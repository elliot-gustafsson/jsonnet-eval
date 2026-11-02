use crate::{arena::allocator::Allocator, interner::Interner};

mod arena;
mod evaluator;
mod interner;

fn main() {
    let allocator = Allocator::new();
    let mut interner = Interner::new();

    let stuff = allocator.alloc::<&str>(3);

    stuff[0] = "hello";
    stuff[1] = "rusty";
    stuff[2] = "world!";

    let s_ids = allocator.alloc::<u32>(3);
    for i in 0..stuff.len() {
        s_ids[i] = interner.intern(stuff[i]);
    }

    for i in 0..s_ids.len() {
        println!("{}", interner.get(s_ids[i]));
    }
}
