pub mod arena;
pub mod ast;
pub mod evaluator;
pub mod interner;

use std::error::Error;
use std::fs::File;
use std::io::Read;

use crate::arena::Allocator;
use crate::ast::Ast;
use crate::ast::parse::parse_snippet;
use crate::evaluator::{
    Context, ContextState, EvaluationError, Value, create_file_scope, evaluate_node,
};
use crate::interner::Interner;

pub struct Evaluator<'a> {
    jpaths: Vec<&'a str>,
    max_stack: u32,
    interner: Interner<'a>,
}

impl<'a> Default for Evaluator<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Evaluator<'a> {
    #[inline]
    pub fn new() -> Self {
        Self {
            jpaths: Vec::new(),
            max_stack: 10000,
            interner: Interner::new(),
        }
    }

    pub fn evaluate_json(self, file: &str) -> Result<&str, EvaluationError> {
        let allocator = Allocator::new();

        let ctx_state = ContextState {
            max_stack: self.max_stack,
            interner: &self.interner,
            allocator: &allocator,
        };

        let ctx = Context::new(&ctx_state);

        let mut file_handle = File::open(file)?;
        let len = file_handle.metadata()?.len() as usize;
        let buf = allocator.alloc::<u8>(len);
        file_handle.read_exact(buf)?;

        let (ast, root_idx) = parse_snippet(file, buf, ctx.state.interner)?;

        let scope = create_file_scope(file, Value::NONE, ctx);

        let value = evaluate_node(ast.base(), root_idx, scope, ctx);
        Ok("")
    }
}
