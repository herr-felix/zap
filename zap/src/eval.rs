use crate::env::Env;
use crate::types::{error, ZapExp, ZapResult};

use std::mem;

enum Form {
    List(Vec<ZapExp>, usize),
    If(ZapExp, ZapExp),
    Quote,
}

pub struct Evaluator {
    stack: Vec<Form>,
}

impl Default for Evaluator {
    fn default() -> Self {
        Evaluator {
            stack: Vec::with_capacity(32),
        }
    }
}

#[inline(always)]
fn swap_exp(list: &mut Vec<ZapExp>, idx: usize, exp: ZapExp) -> ZapExp {
    mem::replace(list.get_mut(idx).unwrap(), exp)
}

impl Evaluator {
    #[inline(always)]
    fn push_if_form(&mut self, mut list: Vec<ZapExp>) -> ZapResult {
        match (list.pop(), list.pop(), list.pop(), list.pop(), list.pop()) {
            (Some(else_branch), Some(then_branch), Some(head), Some(_), None) => {
                self.stack.push(Form::If(then_branch, else_branch));
                Ok(head)
            }
            _ => Err(error("an if form must contain 3 expressions.")),
        }
    }

    #[inline(always)]
    fn push_quote_form(&mut self, mut list: Vec<ZapExp>) -> ZapResult {
        match list.len() {
            2 => {
                self.stack.push(Form::Quote);
                let exp = swap_exp(&mut list, 1, ZapExp::Nil);
                Ok(exp)
            }
            x if x > 2 => Err(error("too many parameteres to quote")),
            _ => Err(error("nothing to quote.")),
        }
    }

    #[inline(always)]
    fn push_list_form(&mut self, mut list: Vec<ZapExp>, idx: usize) -> ZapExp {
        let element = swap_exp(&mut list, idx, ZapExp::Nil);
        self.stack.push(Form::List(list, idx));
        element
    }

    pub async fn eval<E: Env>(&mut self, root: ZapExp, env: &mut E) -> ZapResult {
        self.stack.truncate(0);
        let mut exp = root;

        loop {
            exp = match exp {
                ZapExp::List(list) => match list.first() {
                    Some(ZapExp::Symbol(s)) => match s.as_ref() {
                        "if" => {
                            exp = self.push_if_form(list)?;
                            continue;
                        }
                        "quote" => self.push_quote_form(list)?,
                        _ => {
                            exp = self.push_list_form(list, 0);
                            continue;
                        }
                    },
                    Some(_) => {
                        exp = self.push_list_form(list, 0);
                        continue;
                    }
                    None => ZapExp::List(list),
                },
                ZapExp::Symbol(s) => env.get(&s)?,
                exp => exp,
            };

            loop {
                if let Some(parent) = self.stack.pop() {
                    exp = match parent {
                        Form::List(mut list, mut idx) => {
                            swap_exp(&mut list, idx, exp);
                            idx += 1;
                            if let Some(val) = list.get_mut(idx) {
                                exp = mem::replace(val, ZapExp::Nil);
                                self.stack.push(Form::List(list, idx));
                                break;
                            } else {
                                ZapExp::apply(list).await?
                            }
                        }
                        Form::If(then_branch, else_branch) => {
                            exp = if exp.is_truish() {
                                then_branch
                            } else {
                                else_branch
                            };
                            break;
                        }
                        Form::Quote => exp,
                    };
                } else {
                    return Ok(exp);
                }
            }
        }
    }
}
