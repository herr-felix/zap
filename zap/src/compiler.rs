use crate::env::{symbols, Env};
use crate::vm::{Chunk, Op, RegID};
use crate::zap::{error_msg, Result, Symbol, Value, ZapList};
use std::sync::Arc;

// The compiler takes the expression returned by the reader and return an array of bytecodes
// which can be executed by the VM.

enum ApplyKind {
    Call,
    Add,
}

enum Form {
    Value(Value),
    List(ZapList, u8),
    Apply(ApplyKind, RegID),
    If(ZapList, RegID, Option<Vec<Op>>, Option<Vec<Op>>),
    Do(ZapList, u8, RegID),
    Define(Value, RegID),
}

struct Compiler<'a, E: Env> {
    env: &'a mut E,
    chunk: Chunk,
    forms: Vec<Form>,
    dst: RegID,
    argc: u8,
}

impl<'a, E: Env> Compiler<'a, E> {
    pub fn init(ast: Value, env: &'a mut E) -> Self {
        Compiler {
            env,
            chunk: Chunk::default(),
            forms: vec![Form::Value(ast)],
            dst: 0,
            argc: 0,
        }
    }

    pub fn get_form(&mut self) -> Option<Form> {
        self.forms.pop()
    }

    pub fn chunk(self) -> Arc<Chunk> {
        Arc::new(self.chunk)
    }

    pub fn set_argc(&mut self, argc: u8) {
        self.argc = argc;
    }

    fn load(&mut self, val: &Value) -> Result<()> {
        let const_idx = self.get_const_idx(val)?;
        self.emit(Op::Load {
            dst: self.dst,
            const_idx,
        });
        self.dst += 1;
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
            Value::Symbol(symbols::DO) => {
                if list.len() < 2 {
                    return Err(error_msg("A do form must contains at least 1 parameter"));
                }
                self.forms.push(Form::Do(list, 1, self.dst));
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
                self.forms.push(Form::If(list, self.dst, None, None));
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
        // TODO
        self.load(&Value::Symbol(s))?;
        self.emit(Op::LookUp(self.dst - 1));
        Ok(())
    }

    pub fn eval_define(&mut self, key: &Value, dst: RegID) -> Result<()> {
        self.dst = dst + 1;
        // The "value" should be in reg(dst)
        self.load(key)?;
        self.emit(Op::Define { key: dst + 1, dst });
        Ok(())
    }

    pub fn apply(&mut self, kind: &ApplyKind, start: u8) -> Result<()> {
        let mut argc = self.argc;

        match kind {
            ApplyKind::Call => {
                // Arguments were pushed on the stack
                self.emit(Op::Call {
                    dst: start,
                    start,
                    argc,
                });
            }
            ApplyKind::Add => {
                argc -= 1; // The '+' symbol was not pushed, but was still counted in teh argc
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
        }
        Ok(())
    }

    pub fn then_branch(&mut self, args: ZapList, dst: u8) {
        let branch = args[2].clone();
        self.forms.push(Form::If(
            args,
            dst,
            Some(std::mem::take(&mut self.chunk.ops)),
            None,
        ));
        self.dst = dst;
        self.forms.push(Form::Value(branch));
    }

    pub fn else_branch(&mut self, args: ZapList, dst: u8, chunk: Vec<Op>) {
        let branch = args[3].clone();
        self.forms.push(Form::If(
            args,
            dst,
            Some(chunk),
            Some(std::mem::take(&mut self.chunk.ops)),
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

        let else_jump = (1 + else_branch.len())
            .try_into()
            .map_err(|_| error_msg("Else branch jump is too big."))?;
        self.emit(Op::CondJmp {
            reg: dst,
            n: else_jump,
        });
        self.chunk.ops.extend(else_branch);

        let then_jump = then_branch
            .len()
            .try_into()
            .map_err(|_| error_msg("Then branch jump is too big."))?;
        self.emit(Op::Jmp(then_jump));
        self.chunk.ops.extend(then_branch);

        Ok(())
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
            Form::If(args, start, chunk, then_branch) => {
                match (chunk, then_branch) {
                    (None, None) => {
                        // Then branch
                        compiler.then_branch(args, start);
                    }
                    (Some(chunk), None) => {
                        // Else branch
                        compiler.else_branch(args, start, chunk);
                    }
                    (Some(chunk), Some(then_branch)) => {
                        // Combine the branches in the chunk
                        compiler.combine_branches(start, chunk, then_branch)?;
                    }
                    _ => {}
                }
            }
            Form::Do(list, idx, start) => {
                compiler.eval_next_in_do(list, idx, start);
            }
            Form::Define(symbol, reg) => {
                compiler.eval_define(&symbol, reg)?;
            }
        }
    }

    Ok(compiler.chunk())
}
