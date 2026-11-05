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
    pub parent: Option<&'a Scope<'a>>,
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
    pub fn new_scope(self, parent: Option<&'a Scope<'a>>, length: usize) -> &'a mut Scope<'a> {
        let s = self.state.allocator.create::<Scope<'a>>();
        let bindings = self.state.allocator.alloc::<NamedValue<'a>>(length);

        s.parent = parent;
        s.bindings = bindings;
        s
    }

    pub fn get_scope_bind(self, mut scope: &'a Scope<'a>, key: u32) -> Option<Value<'a>> {
        loop {
            let bindings = &scope.bindings;

            for i in (0..bindings.len()).rev() {
                if bindings[i].key == key {
                    return Some(bindings[i].value);
                }
            }

            let Some(parent) = scope.parent else {
                return None;
            };
            scope = parent;
        }
    }
}

// pub fn create_file_scope(filename: &str, base_std: Value, ctx: Context) -> &Scope {
//     let allocator = ctx.state.allocator;

//     let key_id = ctx.state.interner.intern("thisFile"); // TODO: make this static

//     ctx.new_scope(parent_ptr, length)
// }

pub fn create_file_scope<'a>(
    _filename: &'a str,
    base_std: Value<'a>,
    ctx: Context<'a>,
) -> &'a Scope<'a> {
    let _key_id = ctx.state.interner.intern("thisFile");

    let s = ctx.new_scope(None, 2);
    s.bindings[0] = NamedValue {
        key: ctx.state.interner.intern("$std"),
        value: base_std,
    };
    s.bindings[1] = NamedValue {
        key: ctx.state.interner.intern("std"),
        value: base_std,
    };

    s
}
