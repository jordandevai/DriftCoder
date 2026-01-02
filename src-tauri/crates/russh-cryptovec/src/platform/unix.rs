use libc::c_void;

/// Unlock memory on drop for Unix-based systems.
///
/// On some platforms (notably Android), `mlock/munlock` can fail due to strict
/// `RLIMIT_MEMLOCK` caps or missing permissions. Failing closed here crashes
/// the entire SSH handshake; instead, we fail open and proceed without locked
/// pages.
pub fn munlock(ptr: *const u8, len: usize) {
    unsafe {
        let _ = libc::munlock(ptr as *const c_void, len);
    }
}

pub fn mlock(ptr: *const u8, len: usize) {
    unsafe {
        let _ = libc::mlock(ptr as *const c_void, len);
    }
}

pub fn memset(ptr: *mut u8, value: i32, size: usize) {
    unsafe {
        libc::memset(ptr as *mut c_void, value, size);
    }
}

