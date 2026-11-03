use crate::arena::allocator::Allocator;
use crate::interner::Interner;

use super::value::{NamedValue, Value};

pub struct ContextState<'a> {
    pub max_stack: u32,
    pub interner: &'a Interner<'a>,
    pub allocator: &'a Allocator,
}

#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub state: &'a ContextState<'a>,
    pub self_val: Value<'a>,
    pub super_offset: u32,
    pub depth: u32,
}

pub struct Scope<'a> {
    pub bindings: &'a mut [NamedValue<'a>],
    pub parent_ptr: usize,
}

impl<'a> Context<'a> {
    #[inline]
    pub fn new(state: &'a ContextState<'a>) -> Self {
        Self {
            state,
            self_val: Value::NONE,
            super_offset: 0,
            depth: 0,
        }
    }

    #[inline]
    pub fn new_scope(self, parent_ptr: usize, length: usize) -> (&'a mut Scope<'a>, usize) {
        let s = self.state.allocator.create::<Scope<'a>>();
        let bindings = self.state.allocator.alloc::<NamedValue<'a>>(length);
        // memclr_slice(bindings);

        s.parent_ptr = parent_ptr;
        s.bindings = bindings;

        let ptr_addr = (s as *const Scope<'a>) as usize;
        (s, ptr_addr)
    }

    pub fn get_scope_bind(self, scope_ptr: usize, key: u32) -> Option<Value<'a>> {
        if scope_ptr == 0 {
            return None;
        }

        let mut curr_ptr = scope_ptr;

        loop {
            let scope = unsafe { &*(curr_ptr as *const Scope<'a>) };
            let bindings = &scope.bindings;

            for i in (0..bindings.len()).rev() {
                if bindings[i].key == key {
                    return Some(bindings[i].value);
                }
            }

            if scope.parent_ptr == 0 || scope.parent_ptr == curr_ptr {
                break;
            }

            curr_ptr = scope.parent_ptr;
        }

        None
    }
}
