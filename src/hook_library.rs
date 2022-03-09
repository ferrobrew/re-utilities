use super::{
    detour_binder::{DetourBinder, NonstaticDetourBinder, StaticDetourBinder},
    module::Module,
};

pub struct HookLibrary {
    binders: Vec<&'static StaticDetourBinder>,
    owned_binders: Vec<NonstaticDetourBinder>,

    inits: Vec<Box<dyn Fn(&mut Module) -> anyhow::Result<()>>>,
    enablers: Vec<Box<dyn Fn() -> anyhow::Result<()>>>,
    disablers: Vec<Box<dyn Fn() -> anyhow::Result<()>>>,
}

impl HookLibrary {
    // builder functions
    pub fn new() -> HookLibrary {
        HookLibrary {
            binders: vec![],
            owned_binders: vec![],

            inits: vec![],
            enablers: vec![],
            disablers: vec![],
        }
    }

    pub fn with_binder(mut self, binder: &'static StaticDetourBinder) -> Self {
        self.binders.push(binder);
        self
    }

    pub fn with_detour<F: detour::Function>(
        mut self,
        detour: &'static detour::StaticDetour<F>,
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

    pub fn on_init(
        mut self,
        init_fn: impl Fn(&mut Module) -> anyhow::Result<()> + 'static,
    ) -> Self {
        self.inits.push(Box::new(init_fn));
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
    pub fn init(&mut self, module: &mut Module) -> anyhow::Result<()> {
        for binder in self.binders() {
            binder.bind(module)?;
        }

        for init in &self.inits {
            (*init)(module)?;
        }
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) -> anyhow::Result<()> {
        if enabled {
            for binder in self.binders() {
                binder.enable()?;
            }
            for enabler in &self.enablers {
                (*enabler)()?;
            }
        } else {
            for binder in self.binders() {
                binder.disable()?;
            }
            for disabler in &self.disablers {
                (*disabler)()?;
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
