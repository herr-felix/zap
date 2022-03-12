use crate::env::Env;
use crate::types::{error, ZapErr, ZapExp, ZapFn, ZapList, ZapResult};

enum Form {
    List(ZapList, usize),
    If,
    Do(ZapList, usize),
    Define,
    Quote,
    Let(ZapList, usize),
    Call(usize),
    Return,
}

impl std::fmt::Debug for Form {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Form::List(l, s) => write!(f, "List({:?}, {})", l, s),
            Form::If => write!(f, "If"),
            Form::Do(_, _) => write!(f, "Do"),
            Form::Define => write!(f, "Define"),
            Form::Quote => write!(f, "Quote"),
            Form::Let(_, _) => write!(f, "Let"),
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

    #[inline(always)]
    fn is_in_tail(&self) -> bool {
        self.path.len() > 1 && matches!(self.path[self.path.len() - 1], Form::Return)
    }

    #[inline(always)]
    fn push_quote_form(&mut self, list: ZapList) -> Result<(), ZapErr> {
        match list.len() {
            2 => {
                self.path.push(Form::Quote);
                self.stack.push(list[1].clone());
                Ok(())
            }
            x if x > 2 => Err(error("too many parameteres to quote")),
            _ => Err(error("nothing to quote.")),
        }
    }

    #[inline(always)]
    fn push_let_form(&mut self, list: ZapList) -> Result<(), ZapErr> {
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

                self.path.push(Form::Let(bindings.clone(), 0));

                self.stack.push(list[2].clone());
                self.stack.push(bindings[0].clone());
                Ok(())
            } else {
                Err(error("'let bindings should be a list.'"))
            }
        } else {
            Err(error("'let' needs 2 expressions."))
        }
    }

    #[inline(always)]
    fn push_define_form(&mut self, list: ZapList) -> Result<(), ZapErr> {
        match list.len() {
            3 => match &list[1] {
                ZapExp::Symbol(_) => {
                    self.path.push(Form::Define);
                    self.stack.push(list[1].clone());
                    self.stack.push(list[2].clone());
                    Ok(())
                }
                _ => Err(error("'define' first form must be a symbol")),
            },
            x if x > 3 => Err(error("'define' only need a symbol and an expression")),
            _ => Err(error("'define' needs a symbol and an expression")),
        }
    }

    #[inline(always)]
    fn register_fn(&mut self, list: ZapList) -> Result<(), ZapErr> {
        if list.len() != 3 {
            return Err(error("'fn' needs 2 forms: the parameters and a body."));
        }

        if let ZapExp::List(args) = &list[1] {
            if args.iter().any(|arg| !matches!(arg, ZapExp::Symbol(_))) {
                return Err(error(
                    "'fn': only symbols can be used as function arguments.",
                ));
            }
            self.stack
                .push(ZapFn::new_fn(args.clone(), list[2].clone()));
            Ok(())
        } else {
            Err(error("'fn' first form should be a list of symbols."))
        }
    }

    pub fn eval(&mut self, root: ZapExp) -> ZapResult {
        self.path.clear();
        self.stack.clear();

        self.stack.push(root);

        loop {
            match self.stack.pop().unwrap() {
                ZapExp::List(list) => {
                    if list.len() > 0 {
                        match list[0] {
                            ZapExp::Symbol(ref s) => match s.as_ref() {
                                "if" => {
                                    if list.len() != 4 {
                                        return Err(error(
                                            "an if form must contain 3 expressions.",
                                        ));
                                    }
                                    self.stack.push(list[3].clone());
                                    self.stack.push(list[2].clone());
                                    self.stack.push(list[1].clone());
                                    self.path.push(Form::If);
                                    continue;
                                }
                                "let" => self.push_let_form(list)?,
                                "do" => {
                                    if list.len() == 1 {
                                        return Err(error(
                                            "'do' forms needs at least one inner form",
                                        ));
                                    }
                                    self.stack.push(list[1].clone());
                                    self.path.push(Form::Do(list, 1));
                                    continue;
                                }
                                "define" => {
                                    self.push_define_form(list)?;
                                    continue;
                                }
                                "quote" => self.push_quote_form(list)?,
                                "fn" => self.register_fn(list)?,
                                _ => {
                                    self.stack.push(self.env.get(s)?);
                                    self.path.push(Form::List(list, 0));
                                }
                            },
                            _ => {
                                self.stack.push(list[0].clone());
                                self.path.push(Form::List(list, 0));
                            }
                        }
                    } else {
                        self.stack.push(ZapExp::List(list));
                    }
                }
                ZapExp::Symbol(s) => self.stack.push(self.env.get(&s)?),
                atom => self.stack.push(atom),
            };

            loop {
                #[cfg(debug_assertions)]
                println!("PATH: {:?}", self.path);
                #[cfg(debug_assertions)]
                println!("STACK: {:?}\n", self.stack);

                if let Some(parent) = self.path.pop() {
                    match parent {
                        Form::List(list, mut idx) => {
                            idx += 1;
                            if list.len() > idx {
                                self.stack.push(list[idx].clone());
                                self.path.push(Form::List(list, idx));
                                break;
                            } else {
                                self.path.push(Form::Call(list.len()));
                            }
                        }
                        Form::If => {
                            if self.stack.pop().unwrap().is_truish() {
                                self.stack.swap_remove(self.stack.len() - 2);
                            } else {
                                self.stack.truncate(self.stack.len() - 1);
                            };
                            break;
                        }
                        Form::Let(bindings, mut idx) => {
                            if bindings.len() <= idx {
                                // len == idx, we are popping down the path
                                if !self.is_in_tail() {
                                    self.env.pop();
                                }
                            } else if idx % 2 == 0 {
                                // idx is even, so a key is on the top of the stack
                                match self.stack[self.stack.len() - 1] {
                                    ZapExp::Symbol(_) => {
                                        idx += 1;
                                        self.stack.push(bindings[idx].clone());
                                        self.path.push(Form::Let(bindings, idx));
                                    }
                                    _ => {
                                        return Err(error(
                                            "let: Only symbols can be used for keys.",
                                        ))
                                    }
                                }
                            } else {
                                // idx is odd, so val in on the top of the stack
                                self.env.set(
                                    &self.stack[self.stack.len() - 2],
                                    &self.stack[self.stack.len() - 1],
                                )?;

                                self.stack.truncate(self.stack.len() - 2);

                                idx += 1;
                                if bindings.len() > idx {
                                    self.stack.push(bindings[idx].clone());
                                    self.path.push(Form::Let(bindings, idx));
                                    continue;
                                } else {
                                    self.path.push(Form::Let(bindings, idx));
                                }
                            };
                            break;
                        }
                        Form::Define => {
                            let symbol = self.stack.swap_remove(self.stack.len() - 2);
                            let val = &self.stack[self.stack.len() - 1]; // We keep the last because that's what we return
                            self.env.set_global(&symbol, val)?;
                        }
                        Form::Do(list, mut idx) => {
                            idx += 1;
                            self.stack.push(list[idx].clone());
                            if list.len() > (idx + 1) {
                                // All but the last
                                self.path.push(Form::Do(list, idx));
                            }
                            break;
                        }
                        Form::Call(argc) => {
                            let params = &self.stack[self.stack.len() - argc..];

                            let exp = match &params[0] {
                                ZapExp::Func(f) => match &**f {
                                    ZapFn::Native(_, f) => f(&params[1..])?,
                                    ZapFn::Func { args, ast } => {
                                        if !self.is_in_tail() {
                                            // TCO
                                            self.env.push();
                                            self.path.push(Form::Return);
                                        }

                                        for i in 0..args.len() {
                                            self.env.set(&args[i], &params[i + 1])?;
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

                            self.stack.push(exp);
                            break;
                        }
                        Form::Return => {
                            self.env.pop();
                        }
                        Form::Quote => {}
                    };
                } else {
                    return Ok(self.stack.pop().unwrap());
                }
            }
        }
    }
}
