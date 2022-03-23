use crate::env::{symbols, Env};
use crate::vm::{Chunk, Op};
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
    Apply(ApplyKind),
    If(ZapList, Option<Vec<Op>>, Option<Vec<Op>>),
}

struct Compiler {
    chunk: Chunk,
    forms: Vec<Form>,
    next_reg: u8,
    argc: u8,
}

impl Compiler {
    pub fn init(ast: Value) -> Self {
        Compiler {
            chunk: Chunk::default(),
            forms: vec![Form::Value(ast)],
            next_reg: 0,
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

    fn load(&mut self, val: Value) -> Result<()> {
        let const_idx = self.get_const_idx(&val)?;
        self.emit(Op::Load {
            dst: self.next_reg,
            const_idx,
        });
        self.next_reg += 1;
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
        .or_else(|_| Err(error_msg("Too many constants in the constants table")))
    }

    pub fn eval_list(&mut self, list: ZapList) -> Result<()> {
        if list.len() > 255 {
            return Err(error_msg(
                "A function cannot have more than 254 parameters.",
            ));
        }

        match list[0] {
            Value::Symbol(symbols::PLUS) => {
                self.forms.push(Form::Apply(ApplyKind::Add));
                self.forms.push(Form::List(list, 1));
                return Ok(());
            }
            Value::Symbol(symbols::IF) => {
                if list.len() != 4 {
                    return Err(error_msg("An if form must have 3 parameters"));
                }
                let cond = list[1].clone();
                self.forms.push(Form::If(list, None, None));
                self.forms.push(Form::Value(cond));
                return Ok(());
            }
            _ => self.forms.push(Form::Apply(ApplyKind::Call)),
        }
        self.forms.push(Form::List(list, 0));
        Ok(())
    }

    pub fn eval_next_in_list(&mut self, list: ZapList, idx: u8) {
        let item = list[idx as usize].clone();
        self.forms.push(Form::List(list, idx + 1));
        self.forms.push(Form::Value(item));
    }

    pub fn eval_value(&mut self, val: Value) -> Result<()> {
        self.load(val)?;
        Ok(())
    }

    pub fn eval_symbol<E: Env>(&mut self, s: Symbol, _env: &mut E) -> Result<()> {
        // TODO
        self.load(Value::Symbol(s))?;
        Ok(())
    }

    pub fn apply(&mut self, kind: ApplyKind) -> Result<()> {
        let mut argc = self.argc;

        match kind {
            ApplyKind::Call => {
                let start = self.next_reg - argc;
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
                    self.emit(Op::Load { dst: 0, const_idx });
                } else if argc > 1 {
                    argc -= 1;
                    while argc > 0 {
                        self.emit(Op::Add {
                            a: 0,
                            b: argc,
                            dst: 0,
                        });
                        argc -= 1;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn then_branch(&mut self, args: ZapList) {
        let branch = args[2].clone();
        self.forms.push(Form::If(
            args,
            Some(std::mem::take(&mut self.chunk.ops)),
            None,
        ));
        self.forms.push(Form::Value(branch));
        self.next_reg = 0;
    }

    pub fn else_branch(&mut self, args: ZapList, chunk: Vec<Op>) {
        let branch = args[3].clone();
        self.forms.push(Form::If(
            args,
            Some(chunk),
            Some(std::mem::take(&mut self.chunk.ops)),
        ));
        self.forms.push(Form::Value(branch));
        self.next_reg = 0;
    }

    pub fn combine_branches(&mut self, chunk: Vec<Op>, then_branch: Vec<Op>) {
        let else_branch = std::mem::replace(&mut self.chunk.ops, chunk);

        self.emit(Op::CondJmp(1 + else_branch.len()));
        self.chunk.ops.extend(else_branch);
        self.emit(Op::CondJmp(then_branch.len()));
        self.chunk.ops.extend(then_branch);
    }
}

pub fn compile<E: Env>(ast: Value, env: &mut E) -> Result<Arc<Chunk>> {
    let mut compiler = Compiler::init(ast);

    while let Some(form) = compiler.get_form() {
        match form {
            Form::Value(val) => match val {
                Value::List(list) => {
                    if list.len() > 0 {
                        compiler.eval_list(list)?;
                    } else {
                        compiler.eval_value(Value::List(list))?
                    }
                }
                Value::Symbol(s) => compiler.eval_symbol(s, env)?,
                atom => compiler.eval_value(atom)?,
            },
            Form::List(list, idx) => {
                if list.len() > idx.into() {
                    compiler.eval_next_in_list(list, idx)
                } else {
                    compiler.set_argc(idx);
                }
            }
            Form::Apply(kind) => {
                compiler.apply(kind)?;
            }
            Form::If(args, chunk, then_branch) => {
                match (chunk, then_branch) {
                    (None, None) => {
                        // Then branch
                        compiler.then_branch(args);
                    }
                    (Some(chunk), None) => {
                        // Else branch
                        compiler.else_branch(args, chunk);
                    }
                    (Some(chunk), Some(then_branch)) => {
                        // Combine the branches in the chunk
                        compiler.combine_branches(chunk, then_branch);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(compiler.chunk())
}
