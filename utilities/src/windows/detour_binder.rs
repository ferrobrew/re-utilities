use crate::error::Result;

/// Type alias for user-specified error results
pub type UserResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

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
