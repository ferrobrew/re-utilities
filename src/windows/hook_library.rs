use super::{
    detour_binder::{DetourBinder, NonstaticDetourBinder, StaticDetourBinder},
    module::Module,
    patcher::Patcher,
};

use anyhow::Context;

pub struct HookLibrary {
    binders: Vec<&'static StaticDetourBinder>,
    owned_binders: Vec<NonstaticDetourBinder>,
    patches: Vec<(usize, Vec<u8>)>,

    inits: Vec<Box<dyn Fn(&mut Module) -> anyhow::Result<()>>>,
    shutdowns: Vec<Box<dyn Fn() -> anyhow::Result<()>>>,
    enablers: Vec<Box<dyn Fn() -> anyhow::Result<()>>>,
    disablers: Vec<Box<dyn Fn() -> anyhow::Result<()>>>,
}

impl HookLibrary {
    // builder functions
    pub fn new() -> HookLibrary {
        HookLibrary {
            binders: vec![],
            owned_binders: vec![],
            patches: vec![],

            inits: vec![],
            shutdowns: vec![],
            enablers: vec![],
            disablers: vec![],
        }
    }

    pub fn with_binder(mut self, binder: &'static StaticDetourBinder) -> Self {
        self.binders.push(binder);
        self
    }

    pub fn with_detour<F: retour::Function>(
        mut self,
        detour: &'static retour::StaticDetour<F>,
    ) -> Self {
        self.owned_binders.push(NonstaticDetourBinder {
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
        });
        self
    }

    pub fn with_patch(mut self, address: usize, bytes: &[u8]) -> Self {
        self.patches.push((address, bytes.to_owned()));
        self
    }

    pub fn on_init(
        mut self,
        init_fn: impl Fn(&mut Module) -> anyhow::Result<()> + 'static,
    ) -> Self {
        self.inits.push(Box::new(init_fn));
        self
    }

    pub fn on_shutdown(mut self, shutdown_fn: impl Fn() -> anyhow::Result<()> + 'static) -> Self {
        self.shutdowns.push(Box::new(shutdown_fn));
        self
    }

    pub fn on_enable(mut self, enable_fn: impl Fn() -> anyhow::Result<()> + 'static) -> Self {
        self.enablers.push(Box::new(enable_fn));
        self
    }

    pub fn on_disable(mut self, disable_fn: impl Fn() -> anyhow::Result<()> + 'static) -> Self {
        self.disablers.push(Box::new(disable_fn));
        self
    }

    // operation functions
    pub fn init(&self, module: &mut Module) -> anyhow::Result<()> {
        for binder in self.binders() {
            binder.bind(module)?;
        }

        for init in &self.inits {
            (*init)(module)?;
        }
        Ok(())
    }

    pub fn set_enabled(&self, patcher: &mut Patcher, enabled: bool) -> anyhow::Result<()> {
        if enabled {
            for binder in self.binders() {
                binder.enable()?;
            }
            for enabler in &self.enablers {
                (*enabler)()?;
            }
            for (address, patch) in &self.patches {
                unsafe {
                    patcher.patch(*address, patch);
                }
            }
        } else {
            for binder in self.binders() {
                binder.disable()?;
            }
            for disabler in &self.disablers {
                (*disabler)()?;
            }
            for (address, _) in &self.patches {
                unsafe {
                    patcher.unpatch(*address).context("failed to unpatch")?;
                }
            }
        }
        Ok(())
    }

    fn binders(&self) -> impl Iterator<Item = &dyn DetourBinder> {
        self.binders
            .iter()
            .map(|b| *b as &dyn DetourBinder)
            .chain(self.owned_binders.iter().map(|b| b as &dyn DetourBinder))
    }
}

impl Default for HookLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HookLibrary {
    fn drop(&mut self) {
        for shutdown_fn in &self.shutdowns {
            (shutdown_fn)().unwrap();
        }
    }
}

pub struct HookLibraries(Vec<HookLibrary>);
impl HookLibraries {
    pub fn new(libraries: Vec<HookLibrary>) -> HookLibraries {
        HookLibraries(libraries)
    }

    pub fn init(&self, module: &mut Module) -> anyhow::Result<()> {
        for library in &self.0 {
            library.init(module)?;
        }

        Ok(())
    }

    pub fn set_enabled(&self, patcher: &mut Patcher, enabled: bool) -> anyhow::Result<()> {
        for library in &self.0 {
            library.set_enabled(patcher, enabled)?;
        }

        Ok(())
    }

    pub fn init_and_enable(
        self,
        module: &mut Module,
        patcher: &mut Patcher,
    ) -> anyhow::Result<Self> {
        self.init(module)?;
        self.set_enabled(patcher, true)?;
        Ok(self)
    }
}
