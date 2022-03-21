#![cfg(target_os = "windows")]

pub mod detour_binder;
pub mod hook_library;
pub mod launcher;
pub mod module;

mod patcher;
mod thread_suspender;

pub use patcher::Patcher;
pub use thread_suspender::ThreadSuspender;
