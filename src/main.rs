use jsonnet_eval::arena::allocator::Allocator;
use jsonnet_eval::evaluator::{Context, ContextState};
use jsonnet_eval::interner::Interner;

fn main() {
    let state = ContextState {
        max_stack: 10000,
        interner: &Interner::new(),
        allocator: &Allocator::new(),
    };
    let ctx = Context::new(&state);

    let stuff = ctx.state.allocator.alloc::<&str>(3);

    stuff[0] = "hello";
    stuff[1] = "rusty";
    stuff[2] = "world!";

    let s_ids = ctx.state.allocator.alloc::<u32>(3);
    for i in 0..stuff.len() {
        s_ids[i] = ctx.state.interner.intern(stuff[i]);
    }

    for i in 0..s_ids.len() {
        println!("{}", ctx.state.interner.get(s_ids[i]));
    }
}
