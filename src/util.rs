pub unsafe fn make_ptr<U>(ptr: usize) -> *mut U {
    make_ptr_with_offset(ptr, 0)
}

pub unsafe fn make_ptr_with_offset<U>(ptr: usize, offset: isize) -> *mut U {
    (ptr as *mut u8).offset(offset) as *mut U
}
