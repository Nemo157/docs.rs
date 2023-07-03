mod highlight;

#[repr(transparent)]
pub struct StrPtr(u64);

unsafe fn ptr_to_str<'a>(StrPtr(ptr_len): StrPtr) -> Option<Box<str>> {
    if ptr_len == 0 {
        None
    } else {
        let (ptr, len) = (
            usize::try_from(ptr_len >> 32).unwrap() as *mut u8,
            usize::try_from(ptr_len & u64::from(u32::MAX)).unwrap(),
        );
        unsafe {
            Some(std::str::from_boxed_utf8_unchecked(Box::from_raw(
                std::slice::from_raw_parts_mut(ptr, len),
            )))
        }
    }
}

unsafe fn str_to_ptr(s: Box<str>) -> StrPtr {
    let str = Box::into_raw(s);
    unsafe {
        StrPtr(
            u64::try_from((*str).as_mut_ptr() as usize).unwrap() << 32
                | u64::from(u32::try_from((*str).len()).unwrap()),
        )
    }
}

#[no_mangle]
pub unsafe extern "C" fn new_str(len: usize) -> *mut u8 {
    std::alloc::alloc(std::alloc::Layout::from_size_align(len, 1).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn free_str(ptr: *mut u8, len: usize) {
    std::alloc::dealloc(ptr, std::alloc::Layout::from_size_align(len, 1).unwrap())
}

/// # SAFETY
///
/// * `lang` must either be a null pointer + 0, or a pointer and length for a valid &str
/// * `code` must be a pointer and length for a valid &str
/// * return will be either a null pointer + 0, or a pointer and length for a valid &str
#[no_mangle]
pub unsafe extern "C" fn highlight(lang: StrPtr, code: StrPtr) -> StrPtr {
    let lang = unsafe { ptr_to_str(lang) };
    let code = unsafe { ptr_to_str(code).unwrap_unchecked() };
    match highlight::try_with_lang(lang.as_deref(), &code) {
        Ok(s) => unsafe { str_to_ptr(s.into_boxed_str()) },
        Err(_) => StrPtr(0),
    }
}
