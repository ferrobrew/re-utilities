use super::module::Module;

pub struct DetourBinder {
    pub bind: &'static (dyn Send + Sync + Fn(&mut Module) -> anyhow::Result<()>),
    pub enable: &'static (dyn Send + Sync + Fn() -> anyhow::Result<()>),
    pub disable: &'static (dyn Send + Sync + Fn() -> anyhow::Result<()>),
}

impl DetourBinder {
    pub fn bind(&self, module: &mut Module) -> anyhow::Result<()> {
        (self.bind)(module)
    }
    pub fn enable(&self) -> anyhow::Result<()> {
        (self.enable)()
    }
    pub fn disable(&self) -> anyhow::Result<()> {
        (self.disable)()
    }
}
