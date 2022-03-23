use zap::env::Env;
use zap::{error_msg, Result, Value};

fn is_float(args: &[Value]) -> Result<Value> {
    if args.is_empty() {
        return Err(error_msg("'float?' requires at least 1 argument."));
    }
    for v in args {
        match v {
            Value::Number(_) => continue,
            _ => return Ok(Value::Bool(false)),
        }
    }
    Ok(Value::Bool(true))
}

fn is_false(args: &[Value]) -> Result<Value> {
    if args.is_empty() {
        return Err(error_msg("'false?' requires at least 1 argument."));
    }
    for v in args {
        match v {
            Value::Bool(false) => continue,
            _ => return Ok(Value::Bool(false)),
        }
    }
    Ok(Value::Bool(true))
}

pub fn load<E: Env>(env: &mut E) {
    env.reg_fn("float?", is_float);
    env.reg_fn("false?", is_false);
}
