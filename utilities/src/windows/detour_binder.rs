use crate::error::Result;

pub trait DetourBinder {
    fn enable(&self) -> Result<()>;
    fn disable(&self) -> Result<()>;
}

pub struct CompiletimeDetourBinder {
    pub enable: &'static (dyn Send + Sync + Fn() -> Result<()>),
    pub disable: &'static (dyn Send + Sync + Fn() -> Result<()>),
}
impl DetourBinder for CompiletimeDetourBinder {
    fn enable(&self) -> Result<()> {
        (self.enable)()
    }
    fn disable(&self) -> Result<()> {
        (self.disable)()
    }
}

pub struct RuntimeDetourBinder {
    pub enable: Box<dyn Send + Sync + Fn() -> Result<()>>,
    pub disable: Box<dyn Send + Sync + Fn() -> Result<()>>,
}
impl DetourBinder for RuntimeDetourBinder {
    fn enable(&self) -> Result<()> {
        (self.enable)()
    }
    fn disable(&self) -> Result<()> {
        (self.disable)()
    }
}
