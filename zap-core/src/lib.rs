use zap::env::Env;
use zap::types::Utc;
use zap::types::{error, ZapExp, ZapResult};

fn add(args: &[ZapExp]) -> ZapResult {
    match args.len() {
        0 => Ok(ZapExp::Number(0.0)),
        1 => Ok(args[0].clone()),
        2 => &args[0] + &args[1],
        _ => {
            let mut iter = args.iter();
            let mut acc = (iter.next().unwrap() + iter.next().unwrap())?;
            for arg in iter {
                acc = (&acc + arg)?;
            }
            Ok(acc)
        }
    }
}

fn sub(args: &[ZapExp]) -> ZapResult {
    match args.len() {
        0 => Ok(ZapExp::Number(0.0)),
        1 => Ok(args[0].clone()),
        2 => &args[0] - &args[1],
        _ => {
            let mut iter = args.iter();
            let mut acc = (iter.next().unwrap() - iter.next().unwrap())?;
            for arg in iter {
                acc = (&acc - arg)?;
            }
            Ok(acc)
        }
    }
}

fn mul(args: &[ZapExp]) -> ZapResult {
    match args.len() {
        0 => Ok(ZapExp::Number(0.0)),
        1 => Ok(args[0].clone()),
        2 => &args[0] * &args[1],
        _ => {
            let mut iter = args.iter();
            let mut acc = (iter.next().unwrap() * iter.next().unwrap())?;
            for arg in iter {
                acc = (&acc * arg)?;
            }
            Ok(acc)
        }
    }
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

    let mut result = std::string::String::with_capacity(len);
    for s in strs {
        result.push_str(s);
    }

    Ok(ZapExp::Str(result.into()))
}

fn equal(args: &[ZapExp]) -> ZapResult {
    match args.len() {
        0 => Err(error("'=' requires more than 0 arguments.")),
        1 => Ok(true.into()),
        2 => Ok((args[0] == args[1]).into()),
        _ => {
            let mut iter = args.iter();
            let mut prev = iter.next().unwrap();
            for arg in iter {
                if prev == arg {
                    prev = arg;
                } else {
                    return Ok(false.into());
                }
            }
            Ok(true.into())
        }
    }
}

fn now(_: &[ZapExp]) -> ZapResult {
    Ok(ZapExp::DateTime(Utc::now()))
}

pub fn load<E: Env>(env: &mut E) {
    env.reg_fn("+", add);
    env.reg_fn("-", sub);
    env.reg_fn("*", mul);
    env.reg_fn("float?", is_float);
    env.reg_fn("false?", is_false);
    env.reg_fn("concat", concat);
    env.reg_fn("=", equal);
    env.reg_fn("now", now);
}
