#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Tag {
    None,

    Null,
    True,
    False,
    Number,
    String,

    Var,
    Self_,
    Dollar,
    SuperIndex,
    InSuper,

    Import,
    ImportStr,
    ImportBin,
}

pub struct Ast(Vec<u32>);

// impl Default for Ast {
//     fn default() -> Self {
//         Self::new(1024)
//     }
// }

impl Ast {
    #[inline]
    pub fn new(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    pub fn base(&self) -> *const u32 {
        self.0.as_ptr()
    }

    #[inline(always)]
    pub fn ptr(&self, idx: u32) -> *const u32 {
        unsafe { self.0.as_ptr().add(idx as usize) }
    }

    #[inline(always)]
    pub fn offset(&self) -> u32 {
        self.0.len() as u32
    }

    #[inline(always)]
    pub fn emit_tag(&mut self, tag: Tag) -> u32 {
        let idx = self.0.len() as u32;
        self.0.push(tag as u32);
        idx
    }

    #[inline(always)]
    pub fn emit_tag_u32(&mut self, tag: Tag, v: u32) -> u32 {
        let idx = self.0.len() as u32;
        self.0.extend_from_slice(&[tag as u32, v]);
        idx
    }

    #[inline(always)]
    pub fn emit_number(&mut self, num: f64) -> u32 {
        let idx = self.0.len() as u32;
        let b = num.to_bits();
        self.0
            .extend_from_slice(&[Tag::Number as u32, b as u32, (b >> 32) as u32]);
        idx
    }

    #[inline(always)]
    pub fn read_tag(bp: *const u32, offset: u32) -> Tag {
        unsafe {
            let p = bp.add(offset as usize);
            *(p as *const Tag)
        }
    }

    #[inline(always)]
    pub fn read_u32(bp: *const u32, offset: u32) -> u32 {
        unsafe {
            let p = bp.add(offset as usize);
            *(p as *const u32)
        }
    }

    #[inline(always)]
    pub fn read_f64(bp: *const u32, offset: u32) -> f64 {
        unsafe {
            let p = bp.add(offset as usize);
            (p as *const f64).read_unaligned()
        }
    }
}
