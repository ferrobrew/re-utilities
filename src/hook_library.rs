use super::{detour_binder::DetourBinder, module::Module};

pub struct HookLibrary {
    binders: Vec<&'static DetourBinder>,
    unbound_binders: Vec<&'static DetourBinder>,
    enablers: Vec<Box<dyn Fn(&mut Module) -> anyhow::Result<()>>>,
    disablers: Vec<Box<dyn Fn() -> anyhow::Result<()>>>,
}

impl HookLibrary {
    pub fn new() -> HookLibrary {
        HookLibrary {
            binders: vec![],
            unbound_binders: vec![],
            enablers: vec![],
            disablers: vec![],
        }
    }

    pub fn with_binder(mut self, binder: &'static DetourBinder) -> Self {
        self.unbound_binders.push(binder);
        self
    }

    pub fn with_enable(
        mut self,
        enable_fn: impl Fn(&mut Module) -> anyhow::Result<()> + 'static,
    ) -> Self {
        self.enablers.push(Box::new(enable_fn));
        self
    }

    pub fn with_disable(mut self, disable_fn: impl Fn() -> anyhow::Result<()> + 'static) -> Self {
        self.disablers.push(Box::new(disable_fn));
        self
    }

    fn bind(&mut self, module: &mut Module) -> anyhow::Result<()> {
        for binder in &self.unbound_binders {
            binder.bind(module)?;
        }
        self.binders.append(&mut self.unbound_binders);
        Ok(())
    }

    pub fn set_enabled(&mut self, module: &mut Module, enabled: bool) -> anyhow::Result<()> {
        if enabled {
            self.bind(module)?;

            for binder in &self.binders {
                binder.enable()?;
            }
            for enabler in &self.enablers {
                (*enabler)(module)?;
            }
        } else {
            for binder in &self.binders {
                binder.disable()?;
            }
            for disabler in &self.disablers {
                (*disabler)()?;
            }
        }
        Ok(())
    }
}

impl Default for HookLibrary {
    fn default() -> Self {
        Self::new()
    }
}
