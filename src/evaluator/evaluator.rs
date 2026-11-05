use std::io;

use crate::ast::{Ast, Tag};
use crate::evaluator::evaluator::EvaluationError::UnhandledTagType;
use crate::evaluator::{Context, Scope, Value, ValueType};

use crate::ast::parse::ParseError;

#[derive(Debug)]
pub enum EvaluationError {
    Io(io::Error),
    Parse(String),
    UnhandledTagType(Tag),
    UnhandledValueType(ValueType),
    MaxStackDepthExceeded,
}

impl From<io::Error> for EvaluationError {
    #[inline]
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl<'a> From<ParseError<'a>> for EvaluationError {
    #[inline(always)]
    fn from(err: ParseError<'a>) -> Self {
        Self::Parse(err.to_string())
    }
}

pub fn evaluate_node<'a>(
    bp: *const u32,
    offset: u32,
    scope: &Scope,
    ctx: Context,
) -> Result<Value<'a>, EvaluationError> {
    let val = eval_node(bp, offset, scope, ctx)?; // TODO: map_err WrapErr
    if val.is_thunk() {
        return val.eval(ctx); // TODO: map_err WrapErr
    }
    return Ok(val);
}

fn eval_node_lazy<'a>(
    bp: *const u32,
    offset: u32,
    scope: &Scope,
    ctx: Context,
) -> Result<Value<'a>, EvaluationError> {
    return Ok(Value::NONE);
}

fn eval_node<'a>(
    bp: *const u32,
    offset: u32,
    scope: &Scope,
    ctx: Context,
) -> Result<Value<'a>, EvaluationError> {
    let tag = Ast::read_tag(bp, offset);

    match tag {
        Tag::String => {
            let id = Ast::read_u32(bp, offset + 1);
            return Ok(Value::make_string_const(id));
        }
        Tag::Null => Ok(Value::NULL),
        Tag::False => Ok(Value::FALSE),
        Tag::True => Ok(Value::TRUE),
        Tag::Number => {
            let num = Ast::read_f64(bp, offset + 1); // TODO: embed value directly and read that
            return Ok(Value::make_number(num));
        }
        _ => Err(UnhandledTagType(tag)),
    }
}
