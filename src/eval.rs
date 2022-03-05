use crate::env::Env;
use crate::types::{error, ZapExp, ZapResult};

type ExpList = std::vec::IntoIter<ZapExp>;

pub enum Form {
    List(Vec<ZapExp>, ExpList),
    If(ZapExp, ZapExp),
    Quote,
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

#[inline(always)]
fn push_if_form(stack: &mut Vec<Form>, mut rest: ExpList) -> ZapResult {
    match (rest.next(), rest.next(), rest.next(), rest.next()) {
        (Some(head), Some(then_branch), Some(else_branch), None) => {
            stack.push(Form::If(then_branch, else_branch));
            Ok(head)
        },
        _ => Err(error("an if form must contain 3 expressions.")),
    }
}

#[inline(always)]
fn push_quote_form(stack: &mut Vec<Form>, mut rest: ExpList) -> ZapResult {
    match (rest.next(), rest.next()) {
        (Some(exp), None) => {
            stack.push(Form::Quote);
            Ok(exp)
        },
        (None, None) => Err(error("nothing to quote.")),
        _ => Err(error("too many parameteres to quote")),
    }
}


#[inline(always)]
fn push_list_form(stack: &mut Vec<Form>, head: ZapExp, rest: ExpList, len: usize) -> ZapExp {
    stack.push(Form::List(Vec::with_capacity(len), rest));
    head
}

pub fn new_stack(capacity: usize) -> Vec<Form> {
    Vec::with_capacity(capacity)
}

pub fn eval_exp(stack: &mut Vec<Form>, root: ZapExp, env: &mut Env) -> ZapResult {
    stack.truncate(0);
    let mut exp = root;

    loop {
        exp = match exp {
            ZapExp::List(l) => {
                let len = l.len();
                let mut rest = l.into_iter();
                match rest.next() {
                    Some(ZapExp::Symbol(s)) =>
                        match s.as_ref() {
                            "if" => {
                                exp = push_if_form(stack, rest)?;
                                continue
                            },
                            "quote" => {
                                push_quote_form(stack, rest)?
                            },
                            _ => {
                                exp = push_list_form(stack, ZapExp::Symbol(s), rest, len);
                                continue
                            }
                        }
                    Some(head) => {
                        exp = push_list_form(stack, head, rest, len);
                        continue
                    },
                    None => ZapExp::List(Vec::new())
                }
            }
            ZapExp::Symbol(s) => {
                if let Some(val) = env.get(&s) {
                    val
                } else {
                    return Err(error(format!("symbol '{}' not in scope.", s).as_str()));
                }
            }
            exp => exp,
        };

        loop {
            if let Some(parent) = stack.pop() {
                exp = match parent {
                    Form::List(mut dst, mut rest) => {
                        dst.push(exp);
                        if let Some(val) = rest.next() {
                            stack.push(Form::List(dst, rest));
                            val
                        } else {
                            exp = apply_list(dst)?;
                            continue
                        }
                    },
                    Form::If(then_branch, else_branch) => {
                        if exp.is_truish() {
                            then_branch
                        } else {
                            else_branch
                        }
                    },
                    Form::Quote => {
                        exp = exp;
                        continue
                    },
                };
                break
            } else {
                return Ok(exp);
            }
        }
    }
}
