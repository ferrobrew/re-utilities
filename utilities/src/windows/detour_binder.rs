use crate::error::UserCallbackResult;

pub trait DetourBinder {
    fn enable(&self) -> UserCallbackResult<()>;
    fn disable(&self) -> UserCallbackResult<()>;
}

pub struct CompiletimeDetourBinder {
    pub enable: &'static (dyn Send + Sync + Fn() -> UserCallbackResult<()>),
    pub disable: &'static (dyn Send + Sync + Fn() -> UserCallbackResult<()>),
}
impl DetourBinder for CompiletimeDetourBinder {
    fn enable(&self) -> UserCallbackResult<()> {
        (self.enable)()
    }
    fn disable(&self) -> UserCallbackResult<()> {
        (self.disable)()
    }
}

pub struct RuntimeDetourBinder {
    pub enable: Box<dyn Send + Sync + Fn() -> UserCallbackResult<()>>,
    pub disable: Box<dyn Send + Sync + Fn() -> UserCallbackResult<()>>,
}
impl DetourBinder for RuntimeDetourBinder {
    fn enable(&self) -> UserCallbackResult<()> {
        (self.enable)()
    }
    fn disable(&self) -> UserCallbackResult<()> {
        (self.disable)()
    }
}
