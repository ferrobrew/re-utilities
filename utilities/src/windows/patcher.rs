use std::collections::HashMap;

use crate::util;

struct Patch {
    original_bytes: Box<[u8]>,
}

impl Patch {
    fn original_bytes(&self) -> &[u8] {
        &self.original_bytes
    }
}

pub struct Patcher {
    patches: HashMap<usize, Patch>,
}

#[allow(clippy::missing_safety_doc)]
impl Patcher {
    pub fn new() -> Patcher {
        Patcher {
            patches: HashMap::new(),
        }
    }

    pub unsafe fn safe_write(&self, ptr: *mut u8, bytes: &[u8]) {
        use windows::Win32::System::Memory::{
            VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
        };

        let mut old: PAGE_PROTECTION_FLAGS = Default::default();
        let len = bytes.len();

        let _ = VirtualProtect(ptr as _, len, PAGE_EXECUTE_READWRITE, &mut old);
        std::slice::from_raw_parts_mut(ptr, len).copy_from_slice(bytes);
        let _ = VirtualProtect(ptr as _, len, old, &mut old);
    }

    pub unsafe fn patch(&mut self, address: usize, bytes: &[u8]) {
        let addr_ptr = util::make_ptr::<u8>(address);
        self.patches.insert(
            address,
            Patch {
                original_bytes: std::slice::from_raw_parts(addr_ptr, bytes.len()).into(),
            },
        );

        self.safe_write(addr_ptr, bytes)
    }

    pub unsafe fn unpatch(&mut self, address: usize) -> Option<()> {
        let original_bytes = self.patches.get(&address)?.original_bytes().to_owned();
        self.safe_write(util::make_ptr(address), &original_bytes);
        self.patches.remove(&address).map(|_| ())
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

impl Default for Patcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Patcher {
    fn drop(&mut self) {
        for (address, patch) in self.patches.iter() {
            unsafe {
                self.safe_write(util::make_ptr(*address), patch.original_bytes());
            }
        }
    }
}
