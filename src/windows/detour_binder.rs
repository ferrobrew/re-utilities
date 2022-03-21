use super::module::Module;

pub trait DetourBinder {
    fn bind(&self, module: &mut Module) -> anyhow::Result<()>;
    fn enable(&self) -> anyhow::Result<()>;
    fn disable(&self) -> anyhow::Result<()>;
}

pub struct StaticDetourBinder {
    pub bind: &'static (dyn Send + Sync + Fn(&mut Module) -> anyhow::Result<()>),
    pub enable: &'static (dyn Send + Sync + Fn() -> anyhow::Result<()>),
    pub disable: &'static (dyn Send + Sync + Fn() -> anyhow::Result<()>),
}

impl DetourBinder for StaticDetourBinder {
    fn bind(&self, module: &mut Module) -> anyhow::Result<()> {
        (self.bind)(module)
    }
    fn enable(&self) -> anyhow::Result<()> {
        (self.enable)()
    }
    fn disable(&self) -> anyhow::Result<()> {
        (self.disable)()
    }
}

pub struct NonstaticDetourBinder {
    pub enable: Box<dyn Send + Sync + Fn() -> anyhow::Result<()>>,
    pub disable: Box<dyn Send + Sync + Fn() -> anyhow::Result<()>>,
}

impl DetourBinder for NonstaticDetourBinder {
    fn bind(&self, _: &mut Module) -> anyhow::Result<()> {
        Ok(())
    }
    fn enable(&self) -> anyhow::Result<()> {
        (self.enable)()
    }
    fn disable(&self) -> anyhow::Result<()> {
        (self.disable)()
    }
}
