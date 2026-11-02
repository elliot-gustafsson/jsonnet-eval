use std::slice;
use std::str;

use super::rapidhash::hash_string;

const DEFAULT_INTERNER_SIZE: usize = 16384; // Must be a power of 2
const DEFAULT_INTERNER_MASK: u32 = (DEFAULT_INTERNER_SIZE as u32) - 1;

#[derive(Clone, Copy, Default)]
struct Entry {
    hash: u32,
    idx: u32,
}

pub struct Interner<'a> {
    table: Vec<Entry>,
    strings: Vec<&'a str>,
    mask: u32,
    count: u32,
}

impl<'a> Default for Interner<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Interner<'a> {
    pub fn new() -> Self {
        let mut strings = Vec::with_capacity((DEFAULT_INTERNER_SIZE * 3) / 4);
        strings.push(""); // burn 0 index

        Self {
            table: vec![Entry::default(); DEFAULT_INTERNER_SIZE],
            strings,
            mask: DEFAULT_INTERNER_MASK,
            count: 0,
        }
    }

    #[inline]
    pub fn intern(&mut self, s: &'a str) -> u32 {
        let h = hash_string(s) as u32;
        let mask = self.mask as usize;
        let mut idx = (h as usize) & mask;

        loop {
            let e = unsafe { *self.table.get_unchecked(idx) };

            if e.idx == 0 {
                let new_id = self.strings.len() as u32;
                self.strings.push(s);
                unsafe {
                    *self.table.get_unchecked_mut(idx) = Entry {
                        hash: h,
                        idx: new_id,
                    };
                }
                self.count += 1;

                if self.count > (self.mask * 3) >> 2 {
                    self.resize();
                }
                return new_id;
            }

            if e.hash == h && str_eq(unsafe { *self.strings.get_unchecked(e.idx as usize) }, s) {
                return e.idx;
            }

            idx = (idx + 1) & mask;
        }
    }

    #[inline(always)]
    pub fn get(&self, id: u32) -> &'a str {
        if id == 0 || (id as usize) >= self.strings.len() {
            return "";
        }
        unsafe { *self.strings.get_unchecked(id as usize) }
    }

    pub fn reset(&mut self) {
        self.table.fill(Entry::default());
        self.strings.truncate(1);
        self.count = 0;
    }

    #[cold]
    #[inline(never)]
    fn resize(&mut self) {
        let new_mask = (self.mask << 1) | 1;
        let mask = new_mask as usize;
        let mut new_table = vec![Entry::default(); mask + 1];

        for &e in &self.table {
            if e.idx == 0 {
                continue;
            }

            let mut idx = (e.hash as usize) & mask;
            loop {
                let slot = unsafe { new_table.get_unchecked_mut(idx) };
                if slot.idx == 0 {
                    *slot = e;
                    break;
                }
                idx = (idx + 1) & mask;
            }
        }

        self.table = new_table;
        self.mask = new_mask;
    }
}

#[inline(always)]
fn str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    if a.as_ptr() == b.as_ptr() {
        return true;
    }
    a.as_bytes() == b.as_bytes()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_intern() {
        let mut interner = Interner::new();

        let res1 = interner.intern("test1");
        let res2 = interner.intern("test2");

        assert!(interner.get(res1) == "test1");
        assert!(interner.get(res2) == "test2");
    }
}
