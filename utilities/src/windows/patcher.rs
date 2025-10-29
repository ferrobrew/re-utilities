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

        VirtualProtect(ptr as _, len, PAGE_EXECUTE_READWRITE, &mut old).unwrap();
        std::slice::from_raw_parts_mut(ptr, len).copy_from_slice(bytes);
        VirtualProtect(ptr as _, len, old, &mut old).unwrap();
    }

    /// Patches memory at the given address with the provided bytes.
    ///
    /// If a patch already exists at this address, the original bytes from the first patch
    /// are preserved and reused. This ensures that unpatching will always restore the
    /// true original bytes, not bytes from a previous patch.
    ///
    /// # Safety
    ///
    /// - `address` must be a valid memory address
    /// - The memory at `address` must be readable and writable
    /// - `bytes.len()` bytes must be safe to read/write at `address`
    pub unsafe fn patch(&mut self, address: usize, bytes: &[u8]) {
        let addr_ptr = util::make_ptr::<u8>(address);

        // If a patch already exists, reuse its original_bytes
        let original_bytes = if let Some(existing_patch) = self.patches.remove(&address) {
            existing_patch.original_bytes
        } else {
            // No existing patch, read the original bytes from memory
            std::slice::from_raw_parts(addr_ptr, bytes.len()).into()
        };

        self.patches.insert(address, Patch { original_bytes });

        self.safe_write(addr_ptr, bytes)
    }

    /// Removes a patch at the given address, restoring the original bytes.
    ///
    /// Returns `Some(())` if a patch was successfully removed, or `None` if no patch
    /// exists at the given address. The original bytes are restored regardless of how
    /// many times the address was patched, as the first patch's original bytes are
    /// always preserved.
    ///
    /// # Safety
    ///
    /// - `address` must be a valid memory address
    /// - The memory at `address` must be readable and writable
    /// - The patch must have been created with the same `bytes.len()` as the original patch
    pub unsafe fn unpatch(&mut self, address: usize) -> Option<()> {
        let original_bytes = self.patches.get(&address)?.original_bytes().to_owned();
        self.safe_write(util::make_ptr(address), &original_bytes);
        self.patches.remove(&address).map(|_| ())
    }

    /// Replace a 5-byte call (0xE8 CALL rel16/32) at `src` with a call to our destination `dst`.
    ///
    /// On 64-bit platforms, the destination must be within 32-bit range.
    pub unsafe fn replace_call_destination(&mut self, src: usize, dst: usize) -> usize {
        // First, we determine what the original destination of the call was.
        let orig_call_target: *mut i32 = util::make_ptr_with_offset(src, 1);
        let orig_call_dest = (*orig_call_target as isize) + (src as isize) + 5;

        // Next, we generate a new call to our destination.
        let new_call_target = dst - src - 5;
        let new_call_target: i32 = new_call_target.try_into().unwrap_or_else(|_| panic!(
            "call target out of range {src:x?} {dst:x?} {orig_call_dest:x?} {new_call_target:x?} (must be within 32-bit range)"
        ));
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
