use crate::env::Env;
use crate::types::{error, ZapExp, ZapResult};

enum Form {
    List(Vec<ZapExp>, std::vec::IntoIter<ZapExp>),
}

#[inline(always)]
fn apply_list(list: Vec<ZapExp>) -> ZapResult {
    if let Some((first, args)) = list.split_first() {
        return match first {
            ZapExp::Func(_, func) => func(args),
            _ => Err(error("Only functions call be called.")),
        };
    }
    Err(error("Cannot evaluate a empty list."))
}

pub fn eval(root: ZapExp, env: &mut Env) -> ZapResult {
    let mut stack = Vec::with_capacity(32);
    let mut exp = root;

    loop {
        exp = match exp {
            ZapExp::List(l) => {
                let len = l.len();
                let mut src = l.into_iter();
                if let Some(val) = src.next() {
                    stack.push(Form::List(Vec::with_capacity(len), src));
                    exp = val;
                    continue;
                } else {
                    ZapExp::List(Vec::new())
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
                        dst.push(exp);
                        if let Some(val) = src.next() {
                            stack.push(Form::List(dst, src));
                            exp = val;
                            break;
                        } else {
                            apply_list(dst)?
                        }
                    }
                }
            } else {
                return Ok(exp);
            }
        }
    }
}
