use crate::env::{symbols, Env};
use crate::vm::{Chunk, Op, RegID};
use crate::zap::{error_msg, Result, Symbol, Value, ZapFn, ZapList};
use fxhash::FxHashMap;
use std::cmp::max;
use std::sync::Arc;

// The compiler takes the expression returned by the reader and return an array of bytecodes
// which can be executed by the VM.

#[derive(Debug)]
enum ApplyKind {
    Call,
    Add,
    Eq,
}

#[derive(Debug)]
enum Form {
    Value(Value),
    List(ZapList, u8),
    Apply(ApplyKind, RegID),
    IfCond(ZapList, RegID),
    IfThen(ZapList, RegID, Vec<Op>),
    IfElse(ZapList, RegID, Vec<Op>, Vec<Op>),
    Do(ZapList, u8, RegID),
    Define(Value, RegID),
    Return(Chunk, RegID),
}

struct Compiler<'a, E: Env> {
    env: &'a mut E,
    chunk: Chunk,
    forms: Vec<Form>,
    dst: RegID,
    argc: u8,
    locals: Vec<FxHashMap<Symbol, u8>>,
}

impl<'a, E: Env> Compiler<'a, E> {
    pub fn init(ast: Value, env: &'a mut E) -> Self {
        Compiler {
            env,
            chunk: Chunk::default(),
            forms: vec![Form::Value(ast)],
            dst: 0,
            argc: 0,
            locals: vec![FxHashMap::<Symbol, u8>::default()],
        }
    }

    pub fn get_form(&mut self) -> Option<Form> {
        self.forms.pop()
    }

    fn is_last_exp(&self) -> bool {
        for form in self.forms.iter().rev() {
            match form {
                Form::Return(_, _) => return true,
                Form::IfThen(_, _, _) | Form::IfElse(_, _, _, _) => continue,
                _ => return false,
            }
        }
        false
    }

    fn register_local(&mut self, key: &Value) -> Result<()> {
        if let Value::Symbol(symbol) = key {
            self.bumb_dst();
            let locals = self.locals.last_mut().unwrap();
            locals.insert(*symbol, self.dst);
            Ok(())
        } else {
            Err(error_msg("Only symbols can be used as args in fn."))
        }
    }

    fn get_local(&mut self, s: Symbol) -> Option<RegID> {
        self.locals.last().unwrap().get(&s).copied()
    }

    pub fn chunk(mut self) -> Arc<Chunk> {
        self.chunk.used_regs = max(self.chunk.used_regs, 1);
        Arc::new(self.chunk)
    }

    pub fn set_argc(&mut self, argc: u8) {
        self.argc = argc;
    }

    fn bumb_dst(&mut self) {
        self.chunk.used_regs = max(self.dst + 1, self.chunk.used_regs);
        self.dst += 1;
    }

    fn load(&mut self, val: &Value) -> Result<()> {
        let const_idx = self.get_const_idx(val)?;
        self.emit(Op::Load {
            dst: self.dst,
            const_idx,
        });
        self.bumb_dst();
        Ok(())
    }

    fn emit(&mut self, op: Op) {
        self.chunk.ops.push(op);
    }

    fn get_const_idx(&mut self, val: &Value) -> Result<u16> {
        if let Some(idx) = self.chunk.consts.iter().position(|x| x == val) {
            idx
        } else {
            let idx = self.chunk.consts.len();
            self.chunk.consts.push(val.clone());
            idx
        }
        .try_into()
        .map_err(|_| error_msg("Too many constants in the constants table"))
    }

    pub fn eval_list(&mut self, list: ZapList) -> Result<()> {
        if list.len() > 255 {
            return Err(error_msg(
                "A function cannot have more than 254 parameters.",
            ));
        }

        match list[0] {
            Value::Symbol(symbols::PLUS) => {
                self.forms.push(Form::Apply(ApplyKind::Add, self.dst));
                self.forms.push(Form::List(list, 1));
            }
            Value::Symbol(symbols::EQUAL) => {
                self.forms.push(Form::Apply(ApplyKind::Eq, self.dst));
                self.forms.push(Form::List(list, 1));
            }
            Value::Symbol(symbols::DO) => {
                if list.len() < 2 {
                    return Err(error_msg("A do form must contains at least 1 parameter"));
                }
                self.forms.push(Form::Do(list, 1, self.dst));
            }
            Value::Symbol(symbols::FN) => {
                if list.len() != 3 {
                    return Err(error_msg("A fn form must contains 2 parameters"));
                }
                match &list[1] {
                    Value::List(args) => {
                        // We save the current chunk
                        let chunk = std::mem::take(&mut self.chunk);
                        self.forms.push(Form::Return(chunk, self.dst));

                        self.dst = 0;

                        // Set all the params in the locals.
                        for arg in args.iter() {
                            self.register_local(arg)?;
                        }
                        self.forms.push(Form::Value(list[2].clone()));
                    }
                    _ => {
                        return Err(error_msg("fn's first parameter must be a list"));
                    }
                }
            }
            Value::Symbol(symbols::DEFINE) => {
                if list.len() < 2 {
                    return Err(error_msg("A def form must 2 parameters"));
                }
                self.forms.push(Form::Define(list[1].clone(), self.dst));
                self.forms.push(Form::Value(list[2].clone()));
            }
            Value::Symbol(symbols::IF) => {
                if list.len() != 4 {
                    return Err(error_msg("An if form must have 3 parameters"));
                }
                let cond = list[1].clone();
                self.forms.push(Form::IfCond(list, self.dst));
                self.forms.push(Form::Value(cond));
            }
            _ => {
                self.forms.push(Form::Apply(ApplyKind::Call, self.dst));
                self.forms.push(Form::List(list, 0));
            }
        }
        Ok(())
    }

    pub fn eval_next_in_list(&mut self, list: ZapList, idx: u8) {
        let item = list[idx as usize].clone();
        self.forms.push(Form::List(list, idx + 1));
        self.forms.push(Form::Value(item));
    }

    pub fn eval_next_in_do(&mut self, list: ZapList, idx: u8, dst: RegID) {
        let item = list[idx as usize].clone();
        if (list.len() - 1) > idx.into() {
            self.forms.push(Form::Do(list, idx + 1, dst));
        }
        self.forms.push(Form::Value(item));
        self.dst = dst;
    }

    pub fn eval_const(&mut self, val: &Value) -> Result<()> {
        self.load(val)?;
        Ok(())
    }

    pub fn eval_symbol(&mut self, s: Symbol) -> Result<()> {
        if let Some(reg) = self.get_local(s) {
            self.bumb_dst();
            self.emit(Op::Move {
                dst: self.dst,
                src: reg,
            });
        } else {
            self.load(&Value::Symbol(s))?;
            self.emit(Op::LookUp(self.dst - 1));
        }
        Ok(())
    }

    pub fn eval_define(&mut self, key: &Value, dst: RegID) -> Result<()> {
        self.dst = dst;
        self.bumb_dst();
        // The "value" should be in reg(dst)
        self.load(key)?;
        self.emit(Op::Define { key: dst + 1, dst });
        Ok(())
    }

    pub fn apply(&mut self, kind: &ApplyKind, start: u8) -> Result<()> {
        let mut argc = self.argc;

        match kind {
            ApplyKind::Call => {
                dbg!(&self.forms);
                if self.is_last_exp() {
                    self.emit(Op::Tailcall { start, argc });
                } else {
                    self.emit(Op::Call { start, argc });
                }
            }
            ApplyKind::Add => {
                argc -= 1; // The '+' symbol was not pushed, but was still counted in the argc
                if argc == 0 {
                    let const_idx = self.get_const_idx(&Value::Number(0.0))?;
                    self.emit(Op::Load {
                        dst: start,
                        const_idx,
                    });
                } else if argc > 1 {
                    argc -= 1;
                    while argc > 0 {
                        self.emit(Op::Add {
                            a: start,
                            b: start + argc,
                            dst: start,
                        });
                        argc -= 1;
                    }
                }
            }
            ApplyKind::Eq => {
                argc -= 1; // The '=' symbol was not pushed, but was still counted in the argc
                let count = argc;
                if argc == 1 {
                    let const_idx = self.get_const_idx(&Value::Bool(true))?;
                    self.emit(Op::Load {
                        dst: start,
                        const_idx,
                    });
                } else if argc > 1 {
                    while argc > 1 {
                        self.emit(Op::Eq {
                            a: start + count - argc,
                            b: start + count - 1,
                            dst: start,
                        });
                        argc -= 1;
                        if argc > 1 {
                            self.emit(Op::CondJmp {
                                reg: start,
                                n: (u16::from(argc) - 1) * 2 - 1,
                            })
                        }
                    }
                }
            }
        }
        if self.is_last_exp() {
            self.emit(Op::Move { dst: 0, src: start });
        }
        Ok(())
    }

    pub fn eval_then_branch(&mut self, args: ZapList, dst: u8) {
        let branch = args[2].clone();
        self.forms.push(Form::IfThen(
            args,
            dst,
            std::mem::take(&mut self.chunk.ops),
        ));
        self.dst = dst;
        self.forms.push(Form::Value(branch));
    }

    pub fn eval_else_branch(&mut self, args: ZapList, dst: u8, chunk: Vec<Op>) {
        let branch = args[3].clone();
        self.forms.push(Form::IfElse(
            args,
            dst,
            chunk,
            std::mem::take(&mut self.chunk.ops),
        ));
        self.dst = dst;
        self.forms.push(Form::Value(branch));
    }

    pub fn combine_branches(
        &mut self,
        dst: u8,
        chunk: Vec<Op>,
        then_branch: Vec<Op>,
    ) -> Result<()> {
        let else_branch = std::mem::replace(&mut self.chunk.ops, chunk);

        let then_jump = (1 + then_branch.len())
            .try_into()
            .map_err(|_| error_msg("Then branch jump is too big."))?;
        self.emit(Op::CondJmp {
            reg: dst,
            n: then_jump,
        });
        self.chunk.ops.extend(then_branch);

        let else_jump = else_branch
            .len()
            .try_into()
            .map_err(|_| error_msg("Else branch jump is too big."))?;
        self.emit(Op::Jmp(else_jump));
        self.chunk.ops.extend(else_branch);

        Ok(())
    }

    pub fn wrap_fn(&mut self, mut chunk: Chunk, dst: RegID) {
        // Swap the chunks
        std::mem::swap(&mut self.chunk, &mut chunk);
        self.dst = dst;
        self.forms
            .push(Form::Value(ZapFn::from_chunk(Arc::new(chunk))));
    }
}

pub fn compile<E: Env>(ast: Value, env: &mut E) -> Result<Arc<Chunk>> {
    let mut compiler = Compiler::init(ast, env);

    while let Some(form) = compiler.get_form() {
        match form {
            Form::Value(val) => match val {
                Value::List(list) => {
                    if list.is_empty() {
                        compiler.eval_const(&Value::List(list))?;
                    } else {
                        compiler.eval_list(list)?;
                    }
                }
                Value::Symbol(s) => compiler.eval_symbol(s)?,
                atom => compiler.eval_const(&atom)?,
            },
            Form::List(list, idx) => {
                if list.len() > idx.into() {
                    compiler.eval_next_in_list(list, idx);
                } else {
                    compiler.set_argc(idx);
                }
            }
            Form::Apply(kind, start) => {
                compiler.apply(&kind, start)?;
            }
            Form::IfCond(args, start) => {
                // Then branch
                compiler.eval_then_branch(args, start);
            }
            Form::IfThen(args, start, chunk) => {
                // Else branch
                compiler.eval_else_branch(args, start, chunk);
            }
            Form::IfElse(args, start, chunk, then_branch) => {
                // Combine the branches in the chunk
                compiler.combine_branches(start, chunk, then_branch)?;
            }
            Form::Do(list, idx, start) => {
                compiler.eval_next_in_do(list, idx, start);
            }
            Form::Define(symbol, reg) => {
                compiler.eval_define(&symbol, reg)?;
            }
            Form::Return(chunk, dst) => compiler.wrap_fn(chunk, dst),
        }
    }

    Ok(compiler.chunk())
}
