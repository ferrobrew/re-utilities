use super::util;
use std::{ptr, slice};
use windows::Win32::System::Memory::{
    VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
};

struct Patch {
    address: *mut u8,
    original_bytes: Vec<u8>,
}

pub struct Patcher {
    patches: Vec<Patch>,
}

impl Patcher {
    pub fn new() -> Patcher {
        Patcher { patches: vec![] }
    }

    pub unsafe fn safe_write(&self, addr_ptr: *mut u8, bytes: &[u8]) {
        let mut old: PAGE_PROTECTION_FLAGS = Default::default();
        VirtualProtect(
            addr_ptr as *const _,
            bytes.len() as _,
            PAGE_EXECUTE_READWRITE,
            &mut old,
        );
        ptr::copy(bytes.as_ptr(), addr_ptr, bytes.len());
        VirtualProtect(addr_ptr as *mut _, bytes.len() as _, old, &mut old);
    }

    pub unsafe fn patch(&mut self, address: usize, bytes: &[u8]) {
        let addr_ptr = util::make_ptr::<u8>(address);
        self.patches.push(Patch {
            address: addr_ptr,
            original_bytes: slice::from_raw_parts(addr_ptr, bytes.len()).to_vec(),
        });

        self.safe_write(addr_ptr, bytes)
    }

    #[cfg(target_pointer_width = "32")]
    pub unsafe fn replace_call_destination(&mut self, src: usize, dst: usize) -> usize {
        // We are replacing an existing call with a call (assumed 5-bytes) to our own code.
        // First, we determine what the original destination of the call was.
        let orig_call_target: *mut isize = util::make_ptr_with_offset(src, 1);
        let orig_call_dest = *orig_call_target + (src as isize) + 5;

        // Next, we generate a new call to our destination.
        let new_call_target = dst - src - 5;
        let new_bytes: [u8; 5] = {
            let mut bytes = [0; 5];
            bytes[0] = 0xE8;
            bytes[1..].copy_from_slice(&new_call_target.to_le_bytes());
            bytes
        };

        // Finally, we patch the existing call and return the original destination.
        self.patch(src, &new_bytes);
        orig_call_dest as usize
    }
}

impl Drop for Patcher {
    fn drop(&mut self) {
        for patch in self.patches.iter().rev() {
            let bytes = &patch.original_bytes;
            unsafe {
                self.safe_write(patch.address, bytes);
            }
        }
    }
}
