use crate::env::symbols::{self};
use crate::env::Env;
use crate::types::{error, ZapExp, ZapFn, ZapList, ZapResult};

enum Form {
    List(ZapList, usize),
    If(ZapList),
    Do(ZapList, usize),
    Define,
    Quote,
    Let(ZapList, usize, usize),
    Call(usize),
    Return,
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Form::List(l, s) => write!(f, "List({:?}, {})", l, s),
            Form::If(_) => write!(f, "If"),
            Form::Do(_, _) => write!(f, "Do"),
            Form::Define => write!(f, "Define"),
            Form::Quote => write!(f, "Quote"),
            Form::Let(_, _, _) => write!(f, "Let"),
            Form::Call(n) => write!(f, "Call({})", n),
            Form::Return => write!(f, "Return"),
        }
    }
}

pub struct Evaluator<E> {
    path: Vec<Form>,
    stack: Vec<ZapExp>,
    env: E,
}

impl<E: Env> Evaluator<E> {
    pub fn new(env: E) -> Self {
        Evaluator {
            path: Vec::with_capacity(32),
            stack: Vec::with_capacity(32),
            env,
        }
    }

    pub fn get_env(&mut self) -> &mut E {
        &mut self.env
    }

    #[inline(always)]
    fn is_in_tail(&self) -> bool {
        self.path.len() > 1 && matches!(self.path[self.path.len() - 1], Form::Return)
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
    fn push_let_form(&mut self, list: ZapList) -> ZapResult {
        if list.len() == 3 {
            if let ZapExp::List(bindings) = &list[1] {
                if bindings.len() < 2 {
                    return Err(error("let must have at least one key and value to bind."));
                }
                if bindings.len() % 2 == 1 {
                    return Err(error(
                        "let must have even number of keys and values to bind.",
                    ));
                }

                if !self.is_in_tail() {
                    // TCO
                    self.env.push();
                }

                self.path.push(Form::Let(bindings.clone(), 0, 0));

                self.stack.push(list[2].clone());

                Ok(bindings[0].clone())
            } else {
                Err(error("'let bindings should be a list.'"))
            }
        } else {
            Err(error("'let' needs 2 expressions."))
        }
    }

    #[inline(always)]
    fn push_define_form(&mut self, list: ZapList) -> ZapResult {
        match list.len() {
            3 => match &list[1] {
                ZapExp::Symbol(_) => {
                    self.path.push(Form::Define);
                    self.stack.push(list[1].clone());
                    Ok(list[2].clone())
                }
                _ => Err(error("'define' first form must be a symbol")),
            },
            x if x > 3 => Err(error("'define' only need a symbol and an expression")),
            _ => Err(error("'define' needs a symbol and an expression")),
        }
    }

    #[inline(always)]
    fn register_fn(&mut self, list: ZapList) -> ZapResult {
        if list.len() != 3 {
            return Err(error("'fn' needs 2 forms: the parameters and a body."));
        }

        if let ZapExp::List(args) = &list[1] {
            let mut arg_symbols = Vec::with_capacity(args.len());

            for arg in args.iter() {
                if let ZapExp::Symbol(s) = arg {
                    arg_symbols.push(*s);
                } else {
                    return Err(error(
                        "'fn': only symbols can be used as function arguments.",
                    ));
                }
            }

            Ok(ZapFn::new_fn(arg_symbols, list[2].clone()))
        } else {
            Err(error("'fn' first form should be a list of symbols."))
        }
    }

    pub fn eval(&mut self, root: ZapExp) -> ZapResult {
        self.path.clear();
        self.stack.clear();

        let mut top = root;

        loop {
            match top {
                ZapExp::List(list) => {
                    if list.len() > 0 {
                        match list[0] {
                            ZapExp::Symbol(id) => {
                                if id == symbols::IF {
                                    if list.len() != 4 {
                                        return Err(error(
                                            "an if form must contain 3 expressions.",
                                        ));
                                    }
                                    top = list[1].clone();
                                    self.path.push(Form::If(list));
                                    continue;
                                } else if id == symbols::LET {
                                    top = self.push_let_form(list)?
                                } else if id == symbols::DO {
                                    if list.len() == 1 {
                                        return Err(error(
                                            "'do' forms needs at least one inner form",
                                        ));
                                    }
                                    top = list[1].clone();
                                    self.path.push(Form::Do(list, 1));
                                    continue;
                                } else if id == symbols::DEFINE {
                                    top = self.push_define_form(list)?;
                                    continue;
                                } else if id == symbols::QUOTE {
                                    top = self.push_quote_form(list)?
                                } else if id == symbols::FN {
                                    top = self.register_fn(list)?
                                } else {
                                    top = self.env.get(id)?;
                                    self.path.push(Form::List(list, 0));
                                }
                            }
                            _ => {
                                top = list[0].clone();
                                self.path.push(Form::List(list, 0));
                            }
                        }
                    } else {
                        top = ZapExp::List(list);
                    }
                }
                ZapExp::Symbol(s) => {
                    top = self.env.get(s)?;
                }
                atom => {
                    top = atom;
                }
            };

            loop {
                if let Some(parent) = self.path.pop() {
                    match parent {
                        Form::List(list, mut idx) => {
                            self.stack.push(top);
                            idx += 1;
                            if list.len() > idx {
                                top = list[idx].clone();
                                self.path.push(Form::List(list, idx));
                                break;
                            } else {
                                top = ZapExp::Nil;
                                self.path.push(Form::Call(list.len()));
                            }
                        }
                        Form::If(branches) => {
                            if top.is_truish() {
                                top = branches[2].clone();
                            } else {
                                top = branches[3].clone();
                            };
                            break;
                        }
                        Form::Let(bindings, sym, mut idx) => {
                            if bindings.len() <= idx {
                                // len == idx, we are popping down the path
                                if !self.is_in_tail() {
                                    self.env.pop();
                                }
                            } else if idx % 2 == 0 {
                                // idx is even, so a key is on the top of the stack
                                match top {
                                    ZapExp::Symbol(s) => {
                                        idx += 1;
                                        top = bindings[idx].clone();
                                        self.path.push(Form::Let(bindings, s, idx));
                                    }
                                    _ => {
                                        return Err(error(
                                            "let: Only symbols can be used for keys.",
                                        ))
                                    }
                                }
                            } else {
                                // idx is odd, so val in on the top of the stack
                                self.env.set(sym, &top)?;

                                idx += 1;
                                if bindings.len() > idx {
                                    top = bindings[idx].clone();
                                    self.path.push(Form::Let(bindings, sym, idx));
                                    continue;
                                } else {
                                    top = self.stack.pop().unwrap();
                                    self.path.push(Form::Let(bindings, sym, idx));
                                }
                            };
                            break;
                        }
                        Form::Define => {
                            let symbol = self.stack.pop().unwrap();
                            self.env.set_global(&symbol, &top)?;
                        }
                        Form::Do(list, mut idx) => {
                            idx += 1;
                            top = list[idx].clone();
                            if list.len() > (idx + 1) {
                                // All but the last
                                self.path.push(Form::Do(list, idx));
                            }
                            break;
                        }
                        Form::Call(argc) => {
                            let params = &self.stack[self.stack.len() - argc..];

                            top = match &params[0] {
                                ZapExp::Func(f) => match &**f {
                                    ZapFn::Native(_, f) => f(&params[1..])?,
                                    ZapFn::Func { args, ast } => {
                                        if !self.is_in_tail() {
                                            // TCO
                                            self.env.push();
                                            self.path.push(Form::Return);
                                        }

                                        for i in 0..args.len() {
                                            self.env.set(args[i], &params[i + 1])?;
                                        }

                                        ast.clone()
                                    }
                                },
                                _ => {
                                    return Err(error("Only functions can be called."));
                                }
                            };
                            // Clear the args from the stack
                            self.stack.truncate(self.stack.len() - argc);
                            break;
                        }
                        Form::Return => {
                            self.env.pop();
                        }
                        Form::Quote => {}
                    };
                } else {
                    return Ok(top);
                }
            }
        }
    }
}
