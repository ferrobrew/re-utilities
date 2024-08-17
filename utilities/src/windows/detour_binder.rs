pub trait DetourBinder {
    fn enable(&self) -> anyhow::Result<()>;
    fn disable(&self) -> anyhow::Result<()>;
}

pub struct CompiletimeDetourBinder {
    pub enable: &'static (dyn Send + Sync + Fn() -> anyhow::Result<()>),
    pub disable: &'static (dyn Send + Sync + Fn() -> anyhow::Result<()>),
}
impl DetourBinder for CompiletimeDetourBinder {
    fn enable(&self) -> anyhow::Result<()> {
        (self.enable)()
    }
    fn disable(&self) -> anyhow::Result<()> {
        (self.disable)()
    }
}

pub struct RuntimeDetourBinder {
    pub enable: Box<dyn Send + Sync + Fn() -> anyhow::Result<()>>,
    pub disable: Box<dyn Send + Sync + Fn() -> anyhow::Result<()>>,
}
impl DetourBinder for RuntimeDetourBinder {
    fn enable(&self) -> anyhow::Result<()> {
        (self.enable)()
    }
    fn disable(&self) -> anyhow::Result<()> {
        (self.disable)()
    }
}
