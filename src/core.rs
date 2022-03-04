use crate::env::Env;
use crate::types::ZapExp::Number;
use crate::types::{error, ZapExp, ZapResult};

fn plus(args: &[ZapExp]) -> ZapResult {
    let mut sum = 0.0;
    for v in args {
        if let ZapExp::Number(x) = v {
            sum = sum + x;
        } else {
            return Err(error("+ can only add numbers."));
        }
    }
    Ok(Number(sum))
}

fn is_float(args: &[ZapExp]) -> ZapResult {
    if args.len() == 0 {
        return Err(error("'float?' requires at least 1 argument."));
    }
    for v in args {
        match v {
            ZapExp::Number(_) => continue,
            _ => return Ok(ZapExp::Bool(false)),
        }
    }
    return Ok(ZapExp::Bool(true));
}

fn is_false(args: &[ZapExp]) -> ZapResult {
    if args.len() == 0 {
        return Err(error("'false?' requires at least 1 argument."));
    }
    for v in args {
        match v {
            ZapExp::Bool(false) => continue,
            _ => return Ok(ZapExp::Bool(false)),
        }
    }
    return Ok(ZapExp::Bool(true));
}

pub fn load(env: &mut Env) {
    env.reg_fn("+", plus);
    env.reg_fn("float?", is_float);
    env.reg_fn("false?", is_false);
}
