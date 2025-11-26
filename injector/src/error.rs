use std::fmt;

/// Error type for re-utilities-injector operations
#[derive(Debug)]
pub enum Error {
    /// Failed to decompose the filename from the payload path
    FilenameDecomposition,
    /// I/O operation failed
    Io {
        context: Option<String>,
        source: std::io::Error,
    },
    /// Failed to allocate memory in the remote process
    RemoteMemoryAllocation { source: windows::core::Error },
    /// Failed to write to process memory
    WriteMemory { source: windows::core::Error },
    /// Failed to get module handle
    GetModuleHandle {
        module: String,
        source: windows::core::Error,
    },
    /// Failed to get procedure address
    GetProcAddress {
        procedure: String,
        source: windows::core::Error,
    },
    /// Failed to create a remote thread
    CreateRemoteThread {
        context: String,
        source: windows::core::Error,
    },
    /// DLL injection wait failed
    InjectionWaitFailed { result: u32 },
    /// Failed to free remote memory
    FreeMemory { source: windows::core::Error },
    /// Failed to get module file name from remote process
    GetRemoteModuleFileName { source: windows::core::Error },
    /// Failed to load library
    LoadLibrary { source: windows::core::Error },
    /// Invalid export name (contains null byte)
    InvalidExportName { source: std::ffi::NulError },
    /// Failed to locate export in module
    ExportNotFound {
        export_name: String,
        source: windows::core::Error,
    },
    /// Remote call thread timed out
    RemoteCallTimeout,
    /// Remote call thread was abandoned
    RemoteCallAbandoned,
    /// Waiting for remote call thread failed
    RemoteCallWaitFailed { result: u32 },
    /// Failed to create a toolhelp snapshot
    CreateSnapshot { source: windows::core::Error },
    /// Failed to canonicalize a path
    CanonicalizePath { source: std::io::Error },
    /// Failed to get first module from snapshot
    GetFirstModule { source: windows::core::Error },
    /// Failed to spawn a process
    SpawnProcess { source: windows::core::Error },
    /// Failed to locate Steam app
    SteamAppNotFound { app_id: u32 },
    /// Steam locate error
    SteamLocate { source: steamlocate::Error },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::FilenameDecomposition => {
                write!(f, "failed to decompose filename from payload path")
            }
            Error::Io { context, source } => {
                if let Some(ctx) = context {
                    write!(f, "{}: {}", ctx, source)
                } else {
                    write!(f, "I/O error: {}", source)
                }
            }
            Error::RemoteMemoryAllocation { source } => {
                write!(f, "failed to allocate memory in remote process: {}", source)
            }
            Error::WriteMemory { source } => {
                write!(f, "failed to write memory: {}", source)
            }
            Error::GetModuleHandle { module, source } => {
                write!(
                    f,
                    "failed to get handle for module '{}': {}",
                    module, source
                )
            }
            Error::GetProcAddress { procedure, source } => {
                write!(
                    f,
                    "failed to get address for procedure '{}': {}",
                    procedure, source
                )
            }
            Error::CreateRemoteThread { context, source } => {
                write!(
                    f,
                    "failed to create remote thread ({}): {}",
                    context, source
                )
            }
            Error::InjectionWaitFailed { result } => {
                write!(f, "failed to inject DLL: wait returned {}", result)
            }
            Error::FreeMemory { source } => {
                write!(f, "failed to free memory: {}", source)
            }
            Error::GetRemoteModuleFileName { source } => {
                write!(f, "failed to get remote module file name: {}", source)
            }
            Error::LoadLibrary { source } => {
                write!(f, "failed to load library: {}", source)
            }
            Error::InvalidExportName { source } => {
                write!(f, "invalid export name: {}", source)
            }
            Error::ExportNotFound {
                export_name,
                source,
            } => {
                write!(f, "failed to locate export '{}': {}", export_name, source)
            }
            Error::RemoteCallTimeout => {
                write!(f, "remote call thread timed out")
            }
            Error::RemoteCallAbandoned => {
                write!(f, "remote call thread was abandoned")
            }
            Error::RemoteCallWaitFailed { result } => {
                write!(f, "waiting for remote call thread failed: {}", result)
            }
            Error::CreateSnapshot { source } => {
                write!(f, "failed to create snapshot: {}", source)
            }
            Error::CanonicalizePath { source } => {
                write!(f, "failed to canonicalize module path: {}", source)
            }
            Error::GetFirstModule { source } => {
                write!(f, "failed to get first module: {}", source)
            }
            Error::SpawnProcess { source } => {
                write!(f, "failed to spawn process: {}", source)
            }
            Error::SteamAppNotFound { app_id } => {
                write!(f, "failed to locate Steam app {}", app_id)
            }
            Error::SteamLocate { source } => {
                write!(f, "Steam locate error: {}", source)
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::RemoteMemoryAllocation { source } => Some(source),
            Error::WriteMemory { source } => Some(source),
            Error::GetModuleHandle { source, .. } => Some(source),
            Error::GetProcAddress { source, .. } => Some(source),
            Error::CreateRemoteThread { source, .. } => Some(source),
            Error::FreeMemory { source } => Some(source),
            Error::GetRemoteModuleFileName { source } => Some(source),
            Error::LoadLibrary { source } => Some(source),
            Error::InvalidExportName { source } => Some(source),
            Error::ExportNotFound { source, .. } => Some(source),
            Error::CreateSnapshot { source } => Some(source),
            Error::CanonicalizePath { source } => Some(source),
            Error::GetFirstModule { source } => Some(source),
            Error::SpawnProcess { source } => Some(source),
            Error::SteamLocate { source } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Error::Io {
            context: None,
            source,
        }
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(source: std::ffi::NulError) -> Self {
        Error::InvalidExportName { source }
    }
}

impl From<steamlocate::Error> for Error {
    fn from(source: steamlocate::Error) -> Self {
        Error::SteamLocate { source }
    }
}

/// Result type alias for re-utilities-injector operations
pub type Result<T> = std::result::Result<T, Error>;
