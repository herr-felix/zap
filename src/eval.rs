use crate::env::Env;
use crate::types::{error, ZapErr, ZapExp, ZapResult};
use std::collections::VecDeque;

enum Form {
    List(VecDeque<ZapExp>, VecDeque<ZapExp>),
}


fn apply_list(list: &[ZapExp]) -> ZapResult {
    if let Some((first, args)) = list.split_first() {
        return match first {
            ZapExp::Func(_, func) => func(args),
            _ => Err(error("Only functions call be called.")),
        }
    }
    Err(error("Cannot evaluate a empty list."))
}


pub fn eval(root: ZapExp, env: &mut Env) -> ZapResult {
    let mut stack = Vec::with_capacity(32);
    let mut exp = root;

    loop {
        exp = match exp {
            ZapExp::List(mut l) => {
                if let Some(val) = l.pop_front() {
                    stack.push(Form::List(VecDeque::with_capacity(l.len() + 1), l));
                    exp = val;
                    continue;
                } else {
                    ZapExp::List(l)
                }
            }
            ZapExp::Symbol(s) => {
                if let Some(val) = env.get(&s) {
                    val
                } else {
                    return Err(error(format!("Symbol '{}' not in scope.", s).as_str()));
                }
            }
            exp => exp,
        };

        loop {
            if let Some(parent) = stack.pop() {
                exp = match parent {
                    Form::List(mut dst, mut src) => {
                        dst.push_back(exp);
                        if let Some(val) = src.pop_front() {
                            stack.push(Form::List(dst, src));
                            exp = val;
                            break;
                        } else {
                            apply_list(dst.make_contiguous())?
                        }
                    }
                }
            } else {
                return Ok(exp);
            }
        }
    }
}
