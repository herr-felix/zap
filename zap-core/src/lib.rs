use zap::env::Env;
use zap::types::ZapExp::Number;
use zap::types::{error, ZapExp, ZapResult};

fn plus(args: &[ZapExp]) -> ZapResult {
    let mut sum = 0.0;
    for v in args {
        if let ZapExp::Number(x) = v {
            sum += x;
        } else {
            return Err(error("+ can only add numbers."));
        }
    }
    Ok(Number(sum))
}

fn is_float(args: &[ZapExp]) -> ZapResult {
    if args.is_empty() {
        return Err(error("'float?' requires at least 1 argument."));
    }
    for v in args {
        match v {
            ZapExp::Number(_) => continue,
            _ => return Ok(ZapExp::Bool(false)),
        }
    }
    Ok(ZapExp::Bool(true))
}

fn is_false(args: &[ZapExp]) -> ZapResult {
    if args.is_empty() {
        return Err(error("'false?' requires at least 1 argument."));
    }
    for v in args {
        match v {
            ZapExp::Bool(false) => continue,
            _ => return Ok(ZapExp::Bool(false)),
        }
    }
    Ok(ZapExp::Bool(true))
}

fn concat(args: &[ZapExp]) -> ZapResult {
    if args.is_empty() {
        return Err(error("'concat' requires at least 1 argument."));
    }
    let mut len = 0;
    let mut strs = Vec::<&str>::with_capacity(args.len());

    for val in args {
        match val {
            ZapExp::Str(s) => {
                strs.push(s.as_ref());
                len += s.len();
            }
            _ => return Err(error("'concat' can only concatenate strings.")),
        }
    }

    let mut result = String::with_capacity(len);
    for s in strs {
        result.push_str(s);
    }

    Ok(ZapExp::Str(result))
}

pub fn load<E: Env>(env: &mut E) {
    env.reg_fn("+", plus);
    env.reg_fn("float?", is_float);
    env.reg_fn("false?", is_false);
    env.reg_fn("concat", concat);
}
