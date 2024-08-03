use std::{collections, ffi::OsString, io, mem, os::windows::ffi::OsStringExt, path::Path, slice};

use windows::Win32::{
    Foundation::HMODULE,
    System::{
        LibraryLoader::GetModuleFileNameW,
        ProcessStatus::{K32EnumProcessModules, K32GetModuleInformation, MODULEINFO},
        Threading::GetCurrentProcess,
    },
};

use anyhow::anyhow;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CacheKey {
    Regular(String),
    RelativeCallsite(String),
    AfterPtr(String, usize),
}

#[allow(dead_code)]
struct SerializedCache {
    hash: u64,
    entries: Vec<(CacheKey, usize)>,
}

#[derive(Debug, Clone)]
pub struct Module {
    handle: HMODULE,
    path: Option<String>,
    pub base: *mut u8,
    _entry_point: *mut u8,
    image_size: u32,
    image_backup: Vec<u8>,
    cache: collections::HashMap<CacheKey, usize>,
}

impl Module {
    pub fn from_handle(handle: HMODULE) -> Module {
        let mut mod_info = unsafe { std::mem::zeroed() };
        unsafe {
            let _ = K32GetModuleInformation(
                GetCurrentProcess(),
                handle,
                &mut mod_info,
                mem::size_of::<MODULEINFO>() as u32,
            )
            .unwrap();
        }
        Module {
            handle,
            path: {
                let mut buf = [0u16; 1024];
                let size = unsafe { GetModuleFileNameW(handle, &mut buf) } as usize;
                let os = OsString::from_wide(&buf[0..size]);
                os.into_string().ok()
            },
            base: mod_info.lpBaseOfDll as *mut u8,
            _entry_point: mod_info.EntryPoint as *mut u8,
            image_size: mod_info.SizeOfImage,
            image_backup: vec![],
            cache: collections::HashMap::new(),
        }
    }

    pub fn get_all() -> impl Iterator<Item = Module> {
        let process = unsafe { GetCurrentProcess() };
        let mut hmodule = HMODULE::default();
        let hmodule_size = mem::size_of::<HMODULE>() as u32;
        let mut needed = 0u32;
        unsafe {
            K32EnumProcessModules(process, &mut hmodule, hmodule_size, &mut needed).unwrap();
        }
        let mut buf = vec![HMODULE::default(); (needed / hmodule_size) as usize];
        unsafe {
            let _ = K32EnumProcessModules(
                process,
                buf.as_mut_ptr(),
                hmodule_size * (buf.len() as u32),
                &mut needed,
            )
            .unwrap();
        }
        buf.into_iter().map(Module::from_handle)
    }

    pub fn as_bytes_from_memory(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.base as *const u8, self.image_size as usize) }
    }

    #[allow(dead_code)]
    pub fn backup_image(&mut self) {
        self.image_backup = self.as_bytes_from_memory().to_vec();
    }

    pub fn as_bytes(&self) -> &[u8] {
        if self.image_backup.is_empty() {
            self.as_bytes_from_memory()
        } else {
            &self.image_backup
        }
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> anyhow::Result<u64> {
        use std::{collections::hash_map::DefaultHasher, fs::File, hash::Hasher};

        struct HashWriter<T: Hasher>(T);

        impl<T: Hasher> io::Write for HashWriter<T> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.write(buf);
                Ok(buf.len())
            }

            fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
                self.write(buf).map(|_| ())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let input = File::open(
            self.path()
                .ok_or_else(|| anyhow!("can't open module for hashing"))?,
        )?;
        let mut reader = io::BufReader::new(input);

        let mut hw = HashWriter(DefaultHasher::new());
        io::copy(&mut reader, &mut hw)?;

        Ok(hw.0.finish())
    }

    pub fn scan(&mut self, pattern: &str) -> anyhow::Result<*mut u8> {
        let offset = if let Some(offset) = self.cache.get(&CacheKey::Regular(pattern.to_owned())) {
            *offset
        } else {
            patternscan::scan_first_match(io::Cursor::new(self.as_bytes()), pattern)?
                .ok_or_else(|| anyhow!("failed to scan"))?
        };

        self.cache
            .insert(CacheKey::Regular(pattern.to_owned()), offset);

        Ok(self.rel_to_abs_addr(offset))
    }

    pub fn scan_for_relative_callsite(
        &self,
        pattern: &str,
        addr_offset: usize,
    ) -> anyhow::Result<*mut u8> {
        let offset = if let Some(offset) = self
            .cache
            .get(&CacheKey::RelativeCallsite(pattern.to_owned()))
        {
            *offset
        } else {
            let offset = patternscan::scan_first_match(io::Cursor::new(self.as_bytes()), pattern)?
                .ok_or_else(|| anyhow!("failed to scan"))?;
            let base = self.rel_to_abs_addr(offset + addr_offset);
            let call = unsafe { slice::from_raw_parts(base as *const u8, 4) };
            let offset = i32::from_ne_bytes(call.try_into()?) + 4;
            let ptr = unsafe { base.offset(offset as isize) };

            self.abs_to_rel_addr(ptr).try_into()?
        };

        Ok(self.rel_to_abs_addr(offset))
    }

    #[allow(dead_code)]
    pub fn scan_after_ptr(&mut self, base: *const u8, pattern: &str) -> anyhow::Result<*mut u8> {
        let base_offset = self.abs_to_rel_addr(base) as usize;

        let offset = if let Some(offset) = self
            .cache
            .get(&CacheKey::AfterPtr(pattern.to_owned(), base_offset))
        {
            *offset
        } else {
            let slice = &self.as_bytes()[base_offset..];

            let offset_from_base = patternscan::scan_first_match(io::Cursor::new(slice), pattern)?
                .ok_or_else(|| anyhow!("failed to scan"))?;

            base_offset + offset_from_base
        };

        self.cache
            .insert(CacheKey::AfterPtr(pattern.to_owned(), base_offset), offset);

        Ok(self.rel_to_abs_addr(offset))
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref().map(Path::new)
    }

    #[allow(dead_code)]
    pub fn directory(&self) -> Option<&Path> {
        self.path().and_then(Path::parent)
    }

    pub fn filename(&self) -> Option<String> {
        self.path()?
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }

    // consider making these unsafe?
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn abs_to_rel_addr(&self, p: *const u8) -> isize {
        unsafe { p.offset_from(self.base) }
    }

    pub fn rel_to_abs_addr(&self, offset: usize) -> *mut u8 {
        self.rel_to_abs_addr_isize(offset as isize)
    }

    pub fn rel_to_abs_addr_isize(&self, offset: isize) -> *mut u8 {
        unsafe { self.base.offset(offset) }
    }

    #[allow(dead_code)]
    pub fn handle(&self) -> HMODULE {
        self.handle
    }

    #[allow(dead_code)]
    pub fn tls_index(&self) -> u32 {
        struct TlsDirectory {
            _tls_start: *const u8,
            _tls_end: *const u8,
            tls_index: *const u32,
            // rest elided
        }

        unsafe {
            let dir_offset = self.rel_to_abs_addr(0x240) as *const u32;
            let dir = self.rel_to_abs_addr((*dir_offset) as usize) as *const TlsDirectory;
            *((*dir).tls_index)
        }
    }
}
