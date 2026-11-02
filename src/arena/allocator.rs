use std::alloc::{self, Layout};
use std::cell::{Cell, UnsafeCell};
use std::ptr::{self, NonNull};
use std::slice;

/// Standard block size (256 KiB).
const CHUNK_SIZE: usize = 256 * 1024;
/// Threshold above which allocations use the jumbo pool (32 KiB).
const MAX_SMALL_ALLOC: usize = 32 * 1024;
/// Alignment of each 256 KiB chunk.
const CHUNK_ALIGN: usize = 16;

pub struct Allocator {
    base: Cell<*mut u8>,
    offset: Cell<usize>,
    curr: Cell<usize>,
    chunks: UnsafeCell<Vec<NonNull<u8>>>,
    jumbos: UnsafeCell<Vec<(NonNull<u8>, Layout)>>,
}

impl Default for Allocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Allocator {
    pub fn new() -> Self {
        let first_chunk = Self::alloc_chunk();
        let mut chunks = Vec::with_capacity(64);
        chunks.push(first_chunk);

        Self {
            base: Cell::new(first_chunk.as_ptr()),
            offset: Cell::new(0),
            curr: Cell::new(0),
            chunks: UnsafeCell::new(chunks),
            jumbos: UnsafeCell::new(Vec::new()),
        }
    }

    #[inline]
    fn alloc_chunk() -> NonNull<u8> {
        let layout = unsafe { Layout::from_size_align_unchecked(CHUNK_SIZE, CHUNK_ALIGN) };
        let ptr = unsafe { alloc::alloc(layout) };
        NonNull::new(ptr).unwrap_or_else(|| alloc::handle_alloc_error(layout))
    }

    // Create type T in the arena.
    #[inline(always)]
    pub fn create<'a, T>(&'a self) -> &'a mut T {
        let size = size_of::<T>();
        let align = align_of::<T>();

        if size == 0 {
            return unsafe { &mut *NonNull::<T>::dangling().as_ptr() };
        }

        let ptr = self.aligned_alloc(size, align) as *mut T;
        unsafe {
            ptr.write_bytes(0, 1);
            &mut *ptr
        }
    }

    // Allocate a slice of type T in the arena with the provided lenght.
    #[inline(always)]
    pub fn alloc<'a, T>(&'a self, length: usize) -> &'a mut [T] {
        if length == 0 {
            return &mut [];
        }

        let elem_size = size_of::<T>();
        let align = align_of::<T>();

        if elem_size == 0 {
            return unsafe { slice::from_raw_parts_mut(NonNull::<T>::dangling().as_ptr(), length) };
        }

        let total_size = elem_size * length;
        let ptr = self.aligned_alloc(total_size, align) as *mut T;

        unsafe { slice::from_raw_parts_mut(ptr, length) }
    }

    // Allocate raw aligned memory inside the arena
    #[inline(always)]
    pub fn aligned_alloc(&self, size: usize, align: usize) -> *mut u8 {
        debug_assert!(align.is_power_of_two());

        let offset = self.offset.get();
        let aligned_offset = (offset + align - 1) & !(align - 1);
        let end = aligned_offset + size;

        if end > CHUNK_SIZE || size > MAX_SMALL_ALLOC {
            return self.alloc_raw_slow(size, align);
        }

        self.offset.set(end);
        unsafe { self.base.get().add(aligned_offset) }
    }

    // Realloc a slice of type T in the arena with the provided lenght.
    pub fn realloc<'a, T: Copy>(&'a self, slice: &'a mut [T], length: usize) -> &'a mut [T] {
        let old_len = slice.len();
        if length <= old_len {
            return &mut slice[..length];
        }

        let elem_size = size_of::<T>();
        if elem_size == 0 {
            return unsafe { slice::from_raw_parts_mut(NonNull::<T>::dangling().as_ptr(), length) };
        }

        if old_len > 0 {
            let slice_ptr = slice.as_mut_ptr() as *mut u8;
            let old_bytes = old_len * elem_size;
            let slice_end = unsafe { slice_ptr.add(old_bytes) };

            let base = self.base.get();
            let offset = self.offset.get();
            let arena_end = unsafe { base.add(offset) };

            // Check if this slice was the very last allocation in the current chunk
            if slice_ptr >= base && slice_end == arena_end {
                let total_size = length * elem_size;
                let additional_bytes = total_size - old_bytes;
                let new_offset = offset + additional_bytes;

                if total_size <= MAX_SMALL_ALLOC && new_offset <= CHUNK_SIZE {
                    self.offset.set(new_offset);
                    unsafe {
                        return slice::from_raw_parts_mut(slice_ptr as *mut T, length);
                    }
                }
            }
        }

        let new_slice = self.alloc::<T>(length);
        new_slice[..old_len].copy_from_slice(slice);
        new_slice
    }

    pub fn reset(&mut self) {
        let chunks = self.chunks.get_mut();
        self.curr.set(0);
        self.base.set(chunks[0].as_ptr());
        self.offset.set(0);

        let jumbos = self.jumbos.get_mut();
        for (ptr, layout) in jumbos.drain(..) {
            unsafe { alloc::dealloc(ptr.as_ptr(), layout) };
        }
    }

    #[cold]
    #[inline(never)]
    fn alloc_raw_slow(&self, size: usize, align: usize) -> *mut u8 {
        debug_assert!(align.is_power_of_two());

        if size > MAX_SMALL_ALLOC {
            let layout = unsafe { Layout::from_size_align_unchecked(size, align) };
            let ptr = unsafe { alloc::alloc(layout) };
            let non_null = NonNull::new(ptr).unwrap_or_else(|| alloc::handle_alloc_error(layout));
            unsafe {
                (*self.jumbos.get()).push((non_null, layout));
            }
            return non_null.as_ptr();
        }

        let next_curr = self.curr.get() + 1;
        self.curr.set(next_curr);

        let chunk_ptr = unsafe {
            let chunks = &mut *self.chunks.get();
            if next_curr >= chunks.len() {
                chunks.push(Self::alloc_chunk());
            }
            chunks[next_curr].as_ptr()
        };

        self.base.set(chunk_ptr);
        self.offset.set(size);
        chunk_ptr
    }
}

impl Drop for Allocator {
    fn drop(&mut self) {
        let chunk_layout = unsafe { Layout::from_size_align_unchecked(CHUNK_SIZE, CHUNK_ALIGN) };
        for chunk in self.chunks.get_mut().drain(..) {
            unsafe { alloc::dealloc(chunk.as_ptr(), chunk_layout) };
        }
        for (ptr, layout) in self.jumbos.get_mut().drain(..) {
            unsafe { alloc::dealloc(ptr.as_ptr(), layout) };
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_allocator_create() {
        let allocator = Allocator::new();

        #[derive(Default)]
        struct Asdf {
            str: String,
            count: usize,
        }

        let x = allocator.create::<Asdf>();
        *x = Asdf {
            str: "test".to_owned(),
            count: 420,
        };

        assert!(x.str == "test");
        assert!(x.count == 420);
    }

    #[test]
    fn test_allocator_alloc() {
        let allocator = Allocator::new();

        struct Asdf {
            str: String,
            count: usize,
        }

        let x = allocator.alloc::<Asdf>(3);

        for i in 0..x.len() {
            x[i] = Asdf {
                str: format!("asdf_{}", i),
                count: i,
            }
        }

        assert!(x.len() == 3);
        assert!(x[0].str == "asdf_0");
        assert!(x[0].count == 0);
        assert!(x[1].str == "asdf_1");
        assert!(x[1].count == 1);
        assert!(x[2].str == "asdf_2");
        assert!(x[2].count == 2);
    }
}
