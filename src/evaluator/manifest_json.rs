use std::fmt::Write;

use crate::evaluator::{
    Context,
    EvaluationError::{self, MaxStackDepthExceeded, UnhandledValueType},
    Value, ValueType,
    utils::unparse_number,
};

pub struct JsonManifestConfig {
    pub indent_step: String,
    pub newline: String,
    pub key_value_sep: String,
    pub space_comma: bool,
    pub python: bool,
    pub strict_float: bool,
}

pub fn manifest_json<'a>(
    b: &mut String,
    value: Value<'a>,
    ctx: Context<'a>,
    config: &JsonManifestConfig,
) -> Result<(), EvaluationError> {
    manifest_json_inner(b, value, ctx, 0, config)
}

fn manifest_json_inner<'a>(
    b: &mut String,
    value: Value<'a>,
    mut ctx: Context<'a>,
    mut indent_level: usize,
    config: &JsonManifestConfig,
) -> Result<(), EvaluationError> {
    if ctx.depth >= ctx.state.max_stack {
        return Err(MaxStackDepthExceeded);
    }
    ctx.depth += 1;

    let has_newline = config.newline != "";

    let value = value.eval(ctx)?;

    return match value.type_of() {
        ValueType::Number => {
            let data = value.number();
            // if config.strict_float {
            //     let _ = write!(b, "{data}");
            //     return Ok(());
            // }
            unparse_number(b, data);
            Ok(())
        }
        ValueType::Null => {
            if config.python {
                b.push_str("None");
                return Ok(());
            }
            b.push_str("null");
            Ok(())
        }
        _ => return Err(UnhandledValueType(value.type_of())),
    };
}
