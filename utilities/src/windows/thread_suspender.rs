use std::mem;

use anyhow::Context;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD,
        },
        Threading::{
            GetCurrentProcessId, GetCurrentThreadId, OpenThread, ResumeThread, SuspendThread,
            THREAD_ALL_ACCESS,
        },
    },
};

pub struct ThreadSuspender {
    threads: Vec<HANDLE>,
}

impl ThreadSuspender {
    pub fn new() -> anyhow::Result<Self> {
        let process_id = unsafe { GetCurrentProcessId() };
        let handle = unsafe {
            CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, process_id)
                .context("failed to create snapshot of process")?
        };

        fn from_snapshot<Value: Default + Copy + Sized>(
            handle: HANDLE,
            first: unsafe fn(HANDLE, *mut Value) -> windows::core::Result<()>,
            next: unsafe fn(HANDLE, *mut Value) -> windows::core::Result<()>,
        ) -> Vec<Value> {
            unsafe {
                let mut value = Default::default();
                let size = &mut value as *mut Value as *mut u32;
                *size = mem::size_of::<Value>() as u32;
                let mut values = Vec::with_capacity(64);
                if first(handle, &mut value).is_ok() {
                    values.push(value);
                    while next(handle, &mut value).is_ok() {
                        values.push(value);
                    }
                }
                values
            }
        }

        let thread_id = unsafe { GetCurrentThreadId() };
        let threads: Vec<HANDLE> = from_snapshot(handle, Thread32First, Thread32Next)
            .iter()
            .filter(|thread| {
                thread.th32OwnerProcessID == process_id && thread.th32ThreadID != thread_id
            })
            .map(|thread| unsafe { OpenThread(THREAD_ALL_ACCESS, false, thread.th32ThreadID) })
            .collect::<Result<Vec<_>, _>>()?;

        Self::suspend(&threads);
        Ok(Self { threads })
    }

    fn suspend(threads: &[HANDLE]) {
        #[cfg(feature = "debug-console")]
        println!("Suspended {} threads", threads.len());
        for handle in threads {
            unsafe { SuspendThread(*handle) };
        }
    }

    fn resume(threads: &[HANDLE]) {
        #[cfg(feature = "debug-console")]
        println!("Resumed {} threads", threads.len());
        for handle in threads {
            unsafe { ResumeThread(*handle) };
        }
    }

    fn close(threads: &[HANDLE]) {
        #[cfg(feature = "debug-console")]
        println!("Closed {} threads", threads.len());
        for handle in threads {
            unsafe {
                let _ = CloseHandle(*handle);
            };
        }
    }

    pub fn for_block<T>(mut f: impl FnMut() -> anyhow::Result<T>) -> anyhow::Result<T> {
        let _suspender = ThreadSuspender::new()?;
        f()
    }
}

impl Drop for ThreadSuspender {
    fn drop(&mut self) {
        Self::resume(&self.threads);
        Self::close(&self.threads);
    }
}
