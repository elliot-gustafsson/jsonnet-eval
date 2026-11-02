use crate::arena::allocator;

struct ContextState {
    pub max_stack: u32,
    pub allocator: allocator::Allocator,
}

struct Context {
    state: *const ContextState,
}
