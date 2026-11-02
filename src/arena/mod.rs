pub mod allocator;

pub use allocator::*;

// memclr zeroes the memory behind a typed pointer as raw bytes.
#[inline(always)]
pub fn memclr<T>(ptr: &mut T) {
    unsafe {
        std::ptr::write_bytes(ptr as *mut T, 0, 1);
    }
}

// memclrSlice zeroes the memory behind a slice pointer as raw bytes.
#[inline(always)]
pub fn memclr_slice<T>(s: &mut [T]) {
    unsafe {
        std::ptr::write_bytes(s.as_mut_ptr(), 0, s.len());
    }
}
