/// # Safety
/// It makes a pointer out of the address you give it. Pretty unsafe.
pub unsafe fn make_ptr<U>(ptr: usize) -> *mut U {
    make_ptr_with_offset(ptr, 0)
}

/// # Safety
/// It makes a pointer out of the address you give it. Pretty unsafe.
pub unsafe fn make_ptr_with_offset<U>(ptr: usize, offset: isize) -> *mut U {
    (ptr as *mut u8).offset(offset) as *mut U
}

// this should probably be a derive macro
#[macro_export]
macro_rules! singleton {
    ($class_name:ty $(, $arg_name:ident : $arg_type:ty)*) => {
        static mut INSTANCE: Option<$class_name> = None;

        impl $class_name {
            pub fn create($($arg_name : $arg_type),*) -> anyhow::Result<()> {
                unsafe {
                    INSTANCE = Some(<$class_name>::new( $($arg_name),* )?);
                }
                Ok(())
            }

            pub fn destroy() {
                unsafe { std::mem::drop(INSTANCE.take()); }
            }

            #[allow(dead_code)]
            pub fn get() -> Option<&'static $class_name> {
                unsafe { INSTANCE.as_ref() }
            }

            #[allow(dead_code)]
            pub fn get_mut() -> Option<&'static mut $class_name> {
                unsafe { INSTANCE.as_mut() }
            }
        }
    };
}
