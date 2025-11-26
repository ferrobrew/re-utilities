use super::{
    detour_binder::{DetourBinder, RuntimeDetourBinder, UserResult},
    patcher::Patcher,
};

use crate::error::Error;

/// Type alias for results that can contain any error type
pub type BoxedResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct UserCallback {
    enable: Box<dyn Fn() -> UserResult<()> + Send + Sync>,
    disable: Box<dyn Fn() -> UserResult<()> + Send + Sync>,
}

#[allow(clippy::type_complexity)]
pub struct HookLibrary {
    static_binders: Vec<&'static dyn DetourBinder>,
    runtime_binders: Vec<Box<dyn DetourBinder>>,
    user_callbacks: Vec<UserCallback>,
    patches: Vec<(usize, Vec<u8>)>,
}
impl HookLibrary {
    // builder functions
    pub fn new() -> HookLibrary {
        HookLibrary {
            static_binders: vec![],
            runtime_binders: vec![],
            user_callbacks: vec![],
            patches: vec![],
        }
    }
    pub fn with_static_binder(mut self, binder: &'static dyn DetourBinder) -> Self {
        self.static_binders.push(binder);
        self
    }
    pub fn with_runtime_binder(mut self, binder: Box<dyn DetourBinder>) -> Self {
        self.runtime_binders.push(binder);
        self
    }
    pub fn with_detour<F: retour::Function>(
        self,
        detour: &'static retour::GenericDetour<F>,
    ) -> Self {
        self.with_runtime_binder(Box::new(RuntimeDetourBinder {
            enable: Box::new(|| {
                unsafe {
                    detour.enable()?;
                }
                Ok(())
            }),
            disable: Box::new(|| {
                unsafe {
                    detour.disable()?;
                }
                Ok(())
            }),
        }))
    }
    pub fn with_callbacks(
        mut self,
        enable: impl Fn() -> UserResult<()> + Send + Sync + 'static,
        disable: impl Fn() -> UserResult<()> + Send + Sync + 'static,
    ) -> Self {
        self.user_callbacks.push(UserCallback {
            enable: Box::new(enable),
            disable: Box::new(disable),
        });
        self
    }
    pub fn with_patch(mut self, address: usize, bytes: &[u8]) -> Self {
        self.patches.push((address, bytes.to_owned()));
        self
    }

    pub fn set_enabled(&self, patcher: &mut Patcher, enabled: bool) -> BoxedResult<()> {
        if enabled {
            for binder in self.binders() {
                binder.enable()?;
            }
            for cb in &self.user_callbacks {
                (cb.enable)()?;
            }
            for (address, patch) in &self.patches {
                unsafe {
                    patcher.patch(*address, patch);
                }
            }
        } else {
            for (address, _) in &self.patches {
                unsafe {
                    patcher
                        .unpatch(*address)
                        .ok_or(Error::UnpatchFailed { address: *address })?;
                }
            }
            for cb in &self.user_callbacks {
                (cb.disable)()?;
            }
            for binder in self.binders() {
                binder.disable()?;
            }
        }
        Ok(())
    }
}
impl HookLibrary {
    fn binders(&self) -> impl Iterator<Item = &dyn DetourBinder> {
        self.static_binders
            .iter()
            .map(|b| *b as &dyn DetourBinder)
            .chain(self.runtime_binders.iter().map(|b| b.as_ref()))
    }
}
impl Default for HookLibrary {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for HookLibrary {
    fn drop(&mut self) {
        for cb in &self.user_callbacks {
            let _ = (cb.disable)();
        }
        for binder in self.binders() {
            let _ = binder.disable();
        }
    }
}

pub struct HookLibraries(Vec<HookLibrary>);
impl HookLibraries {
    pub fn new(libraries: impl Into<Vec<HookLibrary>>) -> HookLibraries {
        HookLibraries(libraries.into())
    }
    pub fn set_enabled(&self, patcher: &mut Patcher, enabled: bool) -> BoxedResult<()> {
        for library in &self.0 {
            library.set_enabled(patcher, enabled)?;
        }
        Ok(())
    }
    pub fn enable(self, patcher: &mut Patcher) -> BoxedResult<Self> {
        self.set_enabled(patcher, true)?;
        Ok(self)
    }
}
