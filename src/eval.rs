use crate::env::Env;
use crate::types::{error, ZapErr, ZapExp};
use std::collections::VecDeque;

enum Form {
    List(VecDeque<ZapExp>, VecDeque<ZapExp>),
}

pub fn eval(root: ZapExp, env: &mut Env) -> Result<ZapExp, ZapErr> {
    let mut stack = Vec::with_capacity(32);
    let mut exp = root;

    // eval (+ 1 2)
    // push List
    // eval + -> env.get("+")
    // eval 1 -> 1
    // eval 2 -> 2
    // Pop ->
    //

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
                            // We made it to the end of the list.
                            // We shall make the function call here.
                            // But for now, we simply return the list as is.
                            ZapExp::List(dst)
                        }
                    }
                }
            } else {
                return Ok(exp);
            }
        }
    }
}
