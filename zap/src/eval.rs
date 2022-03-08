use crate::env::Env;
use crate::types::{error, ZapExp, ZapResult};
use smartstring::alias::String;

use std::mem;

enum Form {
    List(Vec<ZapExp>, usize),
    If(ZapExp, ZapExp),
    Do(Vec<ZapExp>, usize),
    Define(String),
    Quote,
    Let(Vec<ZapExp>, usize, ZapExp),
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
                let exp = mem::take(list.get_mut(1).unwrap());
                Ok(exp)
            }
            x if x > 2 => Err(error("too many parameteres to quote")),
            _ => Err(error("nothing to quote.")),
        }
    }

    #[inline(always)]
    fn push_list_form(&mut self, mut list: Vec<ZapExp>) -> ZapExp {
        let next = mem::take(list.get_mut(0).unwrap());
        self.stack.push(Form::List(list, 0));
        next
    }

    #[inline(always)]
    fn push_let_form<E: Env>(&mut self, mut list: Vec<ZapExp>, env: &mut E) -> ZapResult {
        match list.len() {
            3 => {
                if let (exp, ZapExp::List(mut bindings)) =
                    (list.pop().unwrap(), list.pop().unwrap())
                {
                    if bindings.len() < 2 {
                        return Err(error("let must have at least one key and value to bind."));
                    }
                    if bindings.len() % 2 == 1 {
                        return Err(error(
                            "let must have even number of keys and values to bind.",
                        ));
                    }
                    env.push();
                    let first = mem::take(bindings.get_mut(0).unwrap()); // We know there is at least 2 in there.
                    self.stack.push(Form::Let(bindings, 0, exp));
                    Ok(first)
                } else {
                    Err(error("'let bindings should be a list.'"))
                }
            }
            _ => Err(error("'let' needs 2 expressions.")),
        }
    }

    #[inline(always)]
    fn push_define_form(&mut self, mut list: Vec<ZapExp>) -> ZapResult {
        match list.len() {
            3 => {
                let exp = list.pop().unwrap();
                match list.pop().unwrap() {
                    ZapExp::Symbol(symbol) => {
                        self.stack.push(Form::Define(symbol));
                        Ok(exp)
                    }
                    _ => Err(error("'define' first form must be a symbol")),
                }
            }
            x if x > 3 => Err(error("'define' only need a symbol and an expression")),
            _ => Err(error("'define' needs a symbol and an expression")),
        }
    }

    #[inline(always)]
    fn push_do_form(&mut self, mut list: Vec<ZapExp>) -> ZapResult {
        if list.len() == 1 {
            return Err(error("'do' forms needs at least one inner form"));
        }
        let first = mem::take(list.get_mut(1).unwrap());
        self.stack.push(Form::Do(list, 1));
        Ok(first)
    }

    pub async fn eval<E: Env>(&mut self, root: ZapExp, env: &mut E) -> ZapResult {
        self.stack.truncate(0);
        let mut exp = root;

        loop {
            exp = match exp {
                ZapExp::List(mut list) => {
                    if let Some(first) = list.first_mut() {
                        match first {
                            ZapExp::Symbol(ref s) => match s.as_ref() {
                                "if" => {
                                    exp = self.push_if_form(list)?;
                                    continue;
                                }
                                "let" => self.push_let_form(list, env)?,
                                "do" => {
                                    exp = self.push_do_form(list)?;
                                    continue;
                                }
                                "define" => {
                                    exp = self.push_define_form(list)?;
                                    continue;
                                }
                                "quote" => self.push_quote_form(list)?,
                                _ => {
                                    env.get(first)?;
                                    self.push_list_form(list)
                                }
                            },
                            _ => {
                                exp = self.push_list_form(list);
                                continue;
                            }
                        }
                    } else {
                        ZapExp::List(list)
                    }
                }
                ZapExp::Symbol(_) => {
                    env.get(&mut exp)?;
                    exp
                }
                exp => exp,
            };

            loop {
                if let Some(parent) = self.stack.pop() {
                    exp = match parent {
                        Form::List(mut list, mut idx) => {
                            exp = mem::replace(list.get_mut(idx).unwrap(), exp);
                            idx += 1;
                            if let Some(next) = list.get_mut(idx) {
                                mem::swap(&mut exp, next);
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
                        Form::Let(mut bindings, mut idx, tail) => {
                            let len = bindings.len();
                            exp = if len == idx {
                                // len == idx, we are popping down the stack
                                env.pop();
                                exp
                            } else if idx % 2 == 0 {
                                // idx is even, so exp is a key
                                if matches!(exp, ZapExp::Symbol(_)) {
                                    idx += 1;
                                    exp = mem::replace(bindings.get_mut(idx).unwrap(), exp);
                                    self.stack.push(Form::Let(bindings, idx, tail));
                                    exp
                                } else {
                                    return Err(error("let: Only symbols can be used for keys."));
                                }
                            } else {
                                // idx is odd, so exp is a value
                                let key = mem::take(bindings.get_mut(idx).unwrap());
                                match (key, exp) {
                                    (ZapExp::Symbol(s), val) => {
                                        idx += 1;
                                        env.set(s, val);
                                        if len == idx {
                                            self.stack.push(Form::Let(
                                                bindings,
                                                idx,
                                                ZapExp::default(),
                                            ));
                                            tail
                                        } else {
                                            exp = mem::take(bindings.get_mut(idx).unwrap());
                                            self.stack.push(Form::Let(bindings, idx, tail));
                                            continue;
                                        }
                                    }
                                    (_, _) => {
                                        return Err(error(
                                            "let: Only symbols can be used for keys.",
                                        ))
                                    }
                                }
                            };
                            break;
                        }
                        Form::Define(symbol) => {
                            env.set(symbol, exp.clone());
                            exp
                        }
                        Form::Do(mut list, mut idx) => {
                            idx += 1;
                            if let Some(val) = list.get_mut(idx) {
                                exp = mem::replace(val, ZapExp::Nil);
                                self.stack.push(Form::Do(list, idx));
                                break;
                            }
                            exp
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
