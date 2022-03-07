use crate::env::Env;
use crate::types::{error, ZapExp, ZapResult};

type ExpList = std::vec::IntoIter<ZapExp>;

enum Form {
    List(Vec<ZapExp>, ExpList),
    If(ZapExp, ZapExp),
    Quote,
    Let(ExpList, Option<String>, Option<ZapExp>),
}

pub struct Evaluator {
    stack: Vec<Form>,
}

impl Evaluator {
    pub fn new() -> Evaluator {
        Evaluator {
            stack: Vec::with_capacity(32),
        }
    }

    #[inline(always)]
    fn push_if_form(&mut self, mut rest: ExpList) -> ZapResult {
        match (rest.next(), rest.next(), rest.next(), rest.next()) {
            (Some(head), Some(then_branch), Some(else_branch), None) => {
                self.stack.push(Form::If(then_branch, else_branch));
                Ok(head)
            }
            _ => Err(error("an if form must contain 3 expressions.")),
        }
    }

    #[inline(always)]
    fn push_quote_form(&mut self, mut rest: ExpList) -> ZapResult {
        match (rest.next(), rest.next()) {
            (Some(exp), None) => {
                self.stack.push(Form::Quote);
                Ok(exp)
            }
            (None, None) => Err(error("nothing to quote.")),
            _ => Err(error("too many parameteres to quote")),
        }
    }

    #[inline(always)]
    fn push_list_form(&mut self, head: ZapExp, rest: ExpList, len: usize) -> ZapExp {
        self.stack.push(Form::List(Vec::with_capacity(len), rest));
        head
    }

    #[inline(always)]
    fn push_let_form(&mut self, mut rest: ExpList) -> ZapResult {
        match (rest.next(), rest.next(), rest.next()) {
            (Some(ZapExp::List(seq)), Some(exp), None) => {
                if seq.len() < 2 {
                    return Err(error("let must have at least one key and value to bind."));
                }
                if seq.len() % 2 == 1 {
                    return Err(error(
                        "let must have even number of keys and values to bind.",
                    ));
                }
                let mut bindings = seq.into_iter();
                let first = bindings.next().unwrap(); // We know there is at least 2 in there.
                self.stack.push(Form::Let(bindings, None, Some(exp)));
                Ok(first)
            }
            (Some(_), Some(_), None) => Err(error(
                "let bindings must be a list containing an even number of keys and values to pair.",
            )),
            _ => Err(error("let can only contain 2 forms")),
        }
    }

    pub async fn eval(&mut self, root: ZapExp, env: &mut Env) -> ZapResult {
        self.stack.truncate(0);
        let mut exp = root;

        loop {
            exp = match exp {
                ZapExp::List(l) => {
                    let len = l.len();
                    let mut rest = l.into_iter();
                    match rest.next() {
                        Some(ZapExp::Symbol(s)) => match s.as_ref() {
                            "if" => {
                                exp = self.push_if_form(rest)?;
                                continue;
                            }
                            "let" => {
                                exp = self.push_let_form(rest)?;
                                continue;
                            }
                            "quote" => self.push_quote_form(rest)?,
                            _ => {
                                exp = self.push_list_form(ZapExp::Symbol(s), rest, len);
                                continue;
                            }
                        },
                        Some(head) => {
                            exp = self.push_list_form(head, rest, len);
                            continue;
                        }
                        None => ZapExp::List(Vec::new()),
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
                if let Some(parent) = self.stack.pop() {
                    exp = match parent {
                        Form::List(mut dst, mut rest) => {
                            dst.push(exp);
                            if let Some(val) = rest.next() {
                                self.stack.push(Form::List(dst, rest));
                                exp = val;
                                break;
                            } else {
                                ZapExp::apply(dst).await?
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
                        Form::Let(mut rest, key, tail) => {
                            match (key, rest.next()) {
                                (Some(key), Some(next_key)) => {
                                    env.set(key, exp); // Set the key pair in the env
                                    self.stack.push(Form::Let(rest, None, None));
                                    exp = next_key
                                }
                                (None, Some(next_value)) => {
                                    if let ZapExp::Symbol(s) = exp {
                                        self.stack.push(Form::Let(rest, Some(s), None));
                                        exp = next_value
                                    } else {
                                        return Err(error(
                                            format!("let: Only symbols can be used for keys.")
                                                .as_str(),
                                        ));
                                    }
                                }
                                (Some(_), None) => {
                                    return Err(error(
                                        format!("let: Odd number of form in key value pair.")
                                            .as_str(),
                                    ))
                                }
                                (None, None) => {
                                    if let Some(tail) = tail {
                                        self.stack.push(Form::Let(rest, None, None));
                                        exp = tail
                                    } else {
                                        // Pop the env scope
                                        exp = exp
                                    }
                                }
                            }
                            break;
                        }
                    };
                } else {
                    return Ok(exp);
                }
            }
        }
    }
}
