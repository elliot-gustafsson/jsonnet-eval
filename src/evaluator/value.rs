use std::alloc::Layout;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::slice;
use std::str;

use crate::arena::Allocator;
use crate::interner::Interner;

const INT_SIZE: usize = size_of::<isize>();
const INT_ALIGN: usize = align_of::<isize>();
const VALUE_SIZE: usize = size_of::<Value>();
const VALUE_ALIGN: usize = align_of::<Value>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValueType {
    None = 0,
    Null = 1,
    String = 2,
    Number = 3,
    Bool = 4,
    Object = 5,
    Array = 6,
    Function = 7,
    NativeFunction = 8,
    Thunk = 9,
}

impl ValueType {
    #[inline]
    pub const fn is_literal(self) -> bool {
        matches!(self, Self::String | Self::Number | Self::Bool | Self::Null)
    }

    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Null => "null",
            Self::String => "string",
            Self::Number => "number",
            Self::Bool => "boolean",
            Self::Object => "object",
            Self::Array => "array",
            Self::Function | Self::NativeFunction => "function",
            Self::Thunk => "thunk",
        }
    }
}

// value represents a nan-boxed 64-bit value.
// bit layout for non-float types:

// 63                   51 50    47 46                               0
// |----------------------|--------|---------------------------------|
// |  float nan boundary  |  type  |             payload             |
// |       (13 bits)      |(4 bits)|            (47 bits)            |
// |----------------------|--------|---------------------------------|

// - bits 63-51: reserved for ieee-754 quiet nan float64 detection.
// - bits 50-47: valuetype enum.
// - bits 46-0 : payload (pointer info, interned string id, 1/0 for bools)

// 	note: lowest bits can be used for flags depending on the alignment
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Value<'a>(u64, PhantomData<&'a ()>);

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Value({:?}, {:#x})", self.type_of(), self.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NamedValue<'a> {
    pub key: u32,
    pub value: Value<'a>,
}

impl<'a> Value<'a> {
    const NAN_TAG: u64 = 0xfff8_0000_0000_0000;
    const TAG_MASK: u64 = 0xffff_8000_0000_0000;
    const FLOAT_THRESHOLD: u64 = 1 << 51;
    const CANONICAL_NAN_BITS: u64 = 0x7ff8_0000_0000_0000;
    const TYPE_SHIFT: u64 = 47;

    pub const NONE: Self = Self::from_raw(0);
    pub const NULL: Self = Self::from_raw((ValueType::Null as u64) << Self::TYPE_SHIFT);
    pub const FALSE: Self = Self::from_raw((ValueType::Bool as u64) << Self::TYPE_SHIFT);
    pub const TRUE: Self = Self::from_raw(((ValueType::Bool as u64) << Self::TYPE_SHIFT) | 1);

    #[inline(always)]
    const fn from_raw(bits: u64) -> Self {
        Self(bits, PhantomData)
    }

    #[inline(always)]
    const fn box_data(t: ValueType, data: u32) -> Self {
        Self::from_raw(((t as u64) << Self::TYPE_SHIFT) | (data as u64))
    }

    #[inline(always)]
    fn box_ptr<T>(t: ValueType, ptr: *const T) -> Self {
        let addr = ptr as usize as u64;
        debug_assert!(addr & Self::TAG_MASK == 0, "pointer exceeds 47-bit payload");
        Self::from_raw(((t as u64) << Self::TYPE_SHIFT) | addr)
    }

    #[inline(always)]
    fn unbox_ptr<T>(self) -> *const T {
        ((self.0 & !Self::TAG_MASK) as usize) as *const T
    }

    // --- Constructors ---

    #[inline(always)]
    pub fn make_string(allocator: &'a Allocator, s: &str) -> Self {
        Self::make_string_from_bytes(allocator, s.as_bytes())
    }

    #[inline(always)]
    pub fn make_string_from_bytes(allocator: &'a Allocator, b: &[u8]) -> Self {
        let n = b.len();

        let total_size = INT_SIZE + n;
        let ptr = allocator.aligned_alloc(total_size, INT_ALIGN);

        unsafe {
            // write len at beginning of block
            *(ptr as *mut usize) = n;

            // write data
            let byte_ptr = ptr.add(INT_SIZE);
            std::ptr::copy_nonoverlapping(b.as_ptr(), byte_ptr, n);
        }

        Self::box_ptr(ValueType::String, ptr)
    }

    #[inline(always)]
    pub fn make_string_concat(allocator: &'a Allocator, v1: &str, v2: &str) -> Self {
        let n1 = v1.len();
        let n2 = v2.len();
        let n = n1 + n2;

        let total_size = INT_SIZE + n;
        let ptr = allocator.aligned_alloc(total_size, INT_ALIGN);

        unsafe {
            // write len at beginning of block
            *(ptr as *mut usize) = n;

            // write data
            let byte_ptr = ptr.add(INT_SIZE);
            std::ptr::copy_nonoverlapping(v1.as_ptr(), byte_ptr, n1);
            std::ptr::copy_nonoverlapping(v2.as_ptr(), byte_ptr.add(n1), n2);
        }

        Self::box_ptr(ValueType::String, ptr)
    }

    #[inline(always)]
    pub const fn make_string_const(id: u32) -> Self {
        // Set lowest bit to 1 to signal it's an interned string
        Self::from_raw(((ValueType::String as u64) << Self::TYPE_SHIFT) | ((id as u64) << 1) | 1)
    }

    #[inline(always)]
    pub fn make_number(v: f64) -> Self {
        let bits = if v.is_nan() {
            Self::CANONICAL_NAN_BITS
        } else {
            v.to_bits()
        };
        Self::from_raw(bits ^ Self::NAN_TAG)
    }

    // #[inline]
    // pub fn string(s: &str, ctx: &Context<'a>) -> Self {
    //     Self::string_from_bytes(s.as_bytes(), ctx)
    // }

    // pub fn string_from_bytes(bytes: &[u8], ctx: &Context<'a>) -> Self {
    //     let n = bytes.len();
    //     let (ptr, buf) = alloc_string_builder(ctx, n);
    //     buf.copy_from_slice(bytes);
    //     finalize_string_builder(ptr, n)
    // }

    // pub fn string_concat(v1: &str, v2: &str, ctx: &Context<'a>) -> Self {
    //     let n = v1.len() + v2.len();
    //     let (ptr, buf) = alloc_string_builder(ctx, n);
    //     buf[..v1.len()].copy_from_slice(v1.as_bytes());
    //     buf[v1.len()..].copy_from_slice(v2.as_bytes());
    //     finalize_string_builder(ptr, n)
    // }

    // pub fn array_sized(n: usize, ctx: &Context<'a>) -> (&'a mut [Value<'a>], Self) {
    //     let total_size = size_of::<usize>() + n * size_of::<Value<'a>>();
    //     let layout = Layout::from_size_align(total_size, align_of::<Value<'a>>()).unwrap();

    //     unsafe {
    //         let ptr = ctx.state.allocator.alloc_layout(layout).as_ptr();
    //         *(ptr as *mut usize) = n;

    //         let slice = if n > 0 {
    //             let elem_ptr = ptr.add(size_of::<usize>()) as *mut Value<'a>;
    //             // Initialize to Value::NONE so the slice is valid to read/write in safe Rust
    //             elem_ptr.write_bytes(0, n);
    //             slice::from_raw_parts_mut(elem_ptr, n)
    //         } else {
    //             &mut []
    //         };

    //         (slice, Self::box_ptr(ValueType::Array, ptr))
    //     }
    // }

    #[inline(always)]
    pub fn type_of(self) -> ValueType {
        let t = self.0 >> Self::TYPE_SHIFT;
        if t >= (Self::FLOAT_THRESHOLD >> Self::TYPE_SHIFT) {
            return ValueType::Number;
        }
        // SAFETY: Tags in 0..=9 correspond directly to ValueType variants.
        unsafe { std::mem::transmute::<u8, ValueType>(t as u8) }
    }

    #[inline(always)]
    const fn payload(self) -> usize {
        (self.0 & !Self::TAG_MASK) as usize
    }

    #[inline]
    pub fn string(self, interner: &'a Interner<'a>) -> &'a str {
        debug_assert_eq!(self.type_of(), ValueType::String);

        if (self.0 & 1) == 1 {
            let id = (self.payload() >> 1) as u32;
            return interner.get(id);
        }

        unsafe {
            let ptr = self.unbox_ptr::<u8>();
            let n = *(ptr as *const usize);
            let bytes = slice::from_raw_parts(ptr.add(INT_SIZE), n);
            str::from_utf8_unchecked(bytes)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_create_string() {
        let allocator = Allocator::new();
        let interner = Interner::new();

        let v = Value::make_string(&allocator, "Hello World!");

        assert_eq!(ValueType::String, v.type_of(),);
        assert_eq!("Hello World!", v.string(&interner),);
    }

    #[test]
    fn test_create_concat_strings() {
        let allocator = Allocator::new();
        let interner = Interner::new();

        let v = Value::make_string_concat(&allocator, "Hello ", "World!");

        assert_eq!(ValueType::String, v.type_of(),);
        assert_eq!("Hello World!", v.string(&interner),);
    }
}
