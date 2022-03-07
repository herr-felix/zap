use crate::env::Env;
use crate::types::{error, ZapExp, ZapResult};

type ExpList = std::vec::IntoIter<ZapExp>;

enum Form {
    List(Vec<ZapExp>, ExpList),
    If(ZapExp, ZapExp),
    Quote,
}

pub struct Evaluator {
    stack: Vec<Form>,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
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

    pub async fn eval<E: Env>(&mut self, root: ZapExp, env: &mut E) -> ZapResult {
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
                ZapExp::Symbol(s) => env.get(&s)?,
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
                    };
                } else {
                    return Ok(exp);
                }
            }
        }
    }
}
