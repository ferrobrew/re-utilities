use std::fmt;

/// Error type for re-utilities operations
#[derive(Debug)]
pub enum Error {
    /// Pattern scan failed to find a match
    PatternScanFailed { context: Option<String> },
    /// Module path could not be retrieved
    ModulePathUnavailable,
    /// Failed to create a thread snapshot
    ThreadSnapshotFailed { source: windows::core::Error },
    /// Failed to open a thread
    ThreadOpenFailed { source: windows::core::Error },
    /// Failed to unpatch at the given address
    UnpatchFailed { address: usize },
    /// Detour operation failed
    DetourFailed { source: retour::Error },
    /// I/O operation failed
    Io {
        context: Option<String>,
        source: std::io::Error,
    },
    /// Pattern scan library error
    PatternScan { source: patternscan::Error },
    /// Array conversion failed
    ArrayConversion {
        source: std::array::TryFromSliceError,
    },
    /// Integer conversion failed
    IntConversion { source: std::num::TryFromIntError },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::PatternScanFailed { context } => {
                if let Some(ctx) = context {
                    write!(f, "pattern scan failed: {}", ctx)
                } else {
                    write!(f, "pattern scan failed")
                }
            }
            Error::ModulePathUnavailable => {
                write!(f, "module path unavailable")
            }
            Error::ThreadSnapshotFailed { source } => {
                write!(f, "failed to create thread snapshot: {}", source)
            }
            Error::ThreadOpenFailed { source } => {
                write!(f, "failed to open thread: {}", source)
            }
            Error::UnpatchFailed { address } => {
                write!(f, "failed to unpatch at address 0x{:x}", address)
            }
            Error::DetourFailed { source } => {
                write!(f, "detour operation failed: {}", source)
            }
            Error::Io { context, source } => {
                if let Some(ctx) = context {
                    write!(f, "{}: {}", ctx, source)
                } else {
                    write!(f, "I/O error: {}", source)
                }
            }
            Error::PatternScan { source } => {
                write!(f, "pattern scan error: {}", source)
            }
            Error::ArrayConversion { source } => {
                write!(f, "array conversion failed: {}", source)
            }
            Error::IntConversion { source } => {
                write!(f, "integer conversion failed: {}", source)
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ThreadSnapshotFailed { source } => Some(source),
            Error::ThreadOpenFailed { source } => Some(source),
            Error::DetourFailed { source } => Some(source),
            Error::Io { source, .. } => Some(source),
            Error::PatternScan { source } => Some(source),
            Error::ArrayConversion { source } => Some(source),
            Error::IntConversion { source } => Some(source),
            _ => None,
        }
    }
}

impl From<retour::Error> for Error {
    fn from(source: retour::Error) -> Self {
        Error::DetourFailed { source }
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

impl From<patternscan::Error> for Error {
    fn from(source: patternscan::Error) -> Self {
        Error::PatternScan { source }
    }
}

impl From<std::array::TryFromSliceError> for Error {
    fn from(source: std::array::TryFromSliceError) -> Self {
        Error::ArrayConversion { source }
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(source: std::num::TryFromIntError) -> Self {
        Error::IntConversion { source }
    }
}

/// Result type alias for re-utilities operations
pub type Result<T> = std::result::Result<T, Error>;
