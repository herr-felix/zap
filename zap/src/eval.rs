use crate::env::Env;
use crate::types::{error, ZapExp, ZapList, ZapResult};
use smartstring::alias::String;

enum Form {
    List(ZapList, usize),
    If(ZapExp, ZapExp),
    Do(ZapList, usize),
    Define(String),
    Quote,
    Let(ZapList, usize, ZapExp),
}

pub struct Evaluator {
    path: Vec<Form>,
    stack: Vec<ZapExp>,
}

impl Default for Evaluator {
    fn default() -> Self {
        Evaluator {
            path: Vec::with_capacity(32),
            stack: Vec::with_capacity(32),
        }
    }
}

impl Evaluator {
    #[inline(always)]
    fn push_if_form(&mut self, list: ZapList) -> ZapResult {
        if list.len() == 4 {
            self.path.push(Form::If(list[2].clone(), list[3].clone()));
            Ok(list[1].clone())
        } else {
            Err(error("an if form must contain 3 expressions."))
        }
    }

    #[inline(always)]
    fn push_quote_form(&mut self, list: ZapList) -> ZapResult {
        match list.len() {
            2 => {
                self.path.push(Form::Quote);
                Ok(list[1].clone())
            }
            x if x > 2 => Err(error("too many parameteres to quote")),
            _ => Err(error("nothing to quote.")),
        }
    }

    #[inline(always)]
    fn push_let_form<E: Env>(&mut self, list: ZapList, env: &mut E) -> ZapResult {
        match list.len() {
            3 => {
                if let ZapExp::List(bindings) = &list[1] {
                    if bindings.len() < 2 {
                        return Err(error("let must have at least one key and value to bind."));
                    }
                    if bindings.len() % 2 == 1 {
                        return Err(error(
                            "let must have even number of keys and values to bind.",
                        ));
                    }
                    env.push();
                    self.path
                        .push(Form::Let(bindings.clone(), 0, list[2].clone()));
                    Ok(bindings[0].clone())
                } else {
                    Err(error("'let bindings should be a list.'"))
                }
            }
            _ => Err(error("'let' needs 2 expressions.")),
        }
    }

    #[inline(always)]
    fn push_define_form(&mut self, list: ZapList) -> ZapResult {
        match list.len() {
            3 => match &list[1] {
                ZapExp::Symbol(symbol) => {
                    self.path.push(Form::Define(symbol.clone()));
                    Ok(list[2].clone())
                }
                _ => Err(error("'define' first form must be a symbol")),
            },
            x if x > 3 => Err(error("'define' only need a symbol and an expression")),
            _ => Err(error("'define' needs a symbol and an expression")),
        }
    }

    #[inline(always)]
    fn push_do_form(&mut self, list: ZapList) -> ZapResult {
        if list.len() == 1 {
            return Err(error("'do' forms needs at least one inner form"));
        }
        let first = list[1].clone();
        self.path.push(Form::Do(list, 1));
        Ok(first)
    }

    pub async fn eval<E: Env>(&mut self, root: ZapExp, env: &mut E) -> ZapResult {
        self.path.truncate(0);
        self.stack.truncate(0);
        let mut exp = root;

        loop {
            exp = match exp {
                ZapExp::List(list) => {
                    if let Some(first) = list.first() {
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
                                    exp = env.get(s)?;
                                    self.path.push(Form::List(list, 0));
                                    exp
                                }
                            },
                            _ => {
                                exp = list[0].clone();
                                self.path.push(Form::List(list, 0));
                                continue;
                            }
                        }
                    } else {
                        ZapExp::List(list)
                    }
                }
                ZapExp::Symbol(s) => env.get(&s)?,
                atom => atom,
            };

            loop {
                if let Some(parent) = self.path.pop() {
                    exp = match parent {
                        Form::List(list, mut idx) => {
                            self.stack.push(exp);

                            idx += 1;
                            if list.len() > idx {
                                exp = list[idx].clone();
                                self.path.push(Form::List(list, idx));
                                break;
                            } else {
                                let args = &self.stack[self.stack.len() - list.len()..];
                                ZapExp::apply(args).await?
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
                        Form::Let(bindings, mut idx, tail) => {
                            let len = bindings.len();
                            exp = if len <= idx {
                                // len == idx, we are popping down the path
                                env.pop();
                                exp
                            } else if idx % 2 == 0 {
                                // idx is even, so exp is a key
                                match exp {
                                    ZapExp::Symbol(_) => {
                                        self.stack.push(exp);
                                        idx += 1;
                                        exp = bindings[idx].clone();
                                        self.path.push(Form::Let(bindings, idx, tail));
                                        exp
                                    }
                                    _ => {
                                        return Err(error(
                                            "let: Only symbols can be used for keys.",
                                        ))
                                    }
                                }
                            } else {
                                // idx is odd, so exp is a value
                                let key = self.stack.pop().unwrap();
                                match (key, exp) {
                                    (ZapExp::Symbol(s), val) => {
                                        idx += 1;
                                        env.set(s, val);
                                        if len == idx {
                                            self.path.push(Form::Let(bindings, idx, ZapExp::Nil));
                                            tail
                                        } else {
                                            exp = bindings[idx].clone();
                                            self.path.push(Form::Let(bindings, idx, tail));
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
                        Form::Do(list, mut idx) => {
                            idx += 1;
                            exp = list[idx].clone();
                            if list.len() > (idx + 1) {
                                // All but the last
                                self.path.push(Form::Do(list, idx));
                            }
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
