use std::fmt;

use super::{
    detour_binder::{DetourBinder, RuntimeDetourBinder},
    patcher::Patcher,
};

use crate::error::{Error, UserCallbackResult};

/// Error type for HookLibrary operations
#[derive(Debug)]
pub enum HookLibraryError {
    /// Standard library error
    Standard(Error),
    /// User callback error
    UserCallback(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for HookLibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookLibraryError::Standard(e) => write!(f, "{}", e),
            HookLibraryError::UserCallback(e) => write!(f, "user callback error: {}", e),
        }
    }
}

impl std::error::Error for HookLibraryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HookLibraryError::Standard(e) => e.source(),
            HookLibraryError::UserCallback(e) => e.source(),
        }
    }
}

#[allow(clippy::type_complexity)]
pub struct HookLibrary {
    static_binders: Vec<&'static dyn DetourBinder>,
    runtime_binders: Vec<Box<dyn DetourBinder>>,
    patches: Vec<(usize, Vec<u8>)>,
}
impl HookLibrary {
    // builder functions
    pub fn new() -> HookLibrary {
        HookLibrary {
            static_binders: vec![],
            runtime_binders: vec![],
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
            enable: Box::new(|| unsafe { detour.enable().map_err(|e| Box::new(e) as _) }),
            disable: Box::new(|| unsafe { detour.disable().map_err(|e| Box::new(e) as _) }),
        }))
    }
    pub fn with_callbacks(
        self,
        enable: impl Fn() -> UserCallbackResult<()> + Send + Sync + 'static,
        disable: impl Fn() -> UserCallbackResult<()> + Send + Sync + 'static,
    ) -> Self {
        self.with_runtime_binder(Box::new(RuntimeDetourBinder {
            enable: Box::new(enable),
            disable: Box::new(disable),
        }))
    }
    pub fn with_patch(mut self, address: usize, bytes: &[u8]) -> Self {
        self.patches.push((address, bytes.to_owned()));
        self
    }

    pub fn set_enabled(
        &self,
        patcher: &mut Patcher,
        enabled: bool,
    ) -> Result<(), HookLibraryError> {
        if enabled {
            for binder in self.binders() {
                binder.enable().map_err(HookLibraryError::UserCallback)?;
            }
            for (address, patch) in &self.patches {
                unsafe {
                    patcher.patch(*address, patch);
                }
            }
        } else {
            for (address, _) in &self.patches {
                unsafe {
                    patcher.unpatch(*address).ok_or_else(|| {
                        HookLibraryError::Standard(Error::UnpatchFailed { address: *address })
                    })?;
                }
            }
            for binder in self.binders() {
                binder.disable().map_err(HookLibraryError::UserCallback)?;
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
    pub fn set_enabled(
        &self,
        patcher: &mut Patcher,
        enabled: bool,
    ) -> Result<(), HookLibraryError> {
        for library in &self.0 {
            library.set_enabled(patcher, enabled)?;
        }
        Ok(())
    }
    pub fn enable(self, patcher: &mut Patcher) -> Result<Self, HookLibraryError> {
        self.set_enabled(patcher, true)?;
        Ok(self)
    }
}
