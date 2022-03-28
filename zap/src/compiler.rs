use crate::env::{symbols, Env};
use crate::vm::{Chunk, Op, RegID};
use crate::zap::{error_msg, Result, Symbol, Value, ZapFn, ZapList};
use fxhash::FxHashMap;
use std::cmp::max;
use std::sync::Arc;

// The compiler takes the expression returned by the reader and return an array of bytecodes
// which can be executed by the VM.

#[derive(Debug)]
enum Form {
    Value(Value),
    List(ZapList, u8),
    Apply,
    IfCond(ZapList),
    IfThen(ZapList, Vec<Op>),
    IfElse(Vec<Op>, Vec<Op>),
    Do(ZapList, usize),
    Define,
    Return(Chunk),
}

struct Compiler<'a, E: Env> {
    env: &'a mut E,
    chunk: Chunk,
    forms: Vec<Form>,
    argc: u8,
    locals: Vec<FxHashMap<Symbol, u8>>,
}

impl<'a, E: Env> Compiler<'a, E> {
    pub fn init(ast: Value, env: &'a mut E) -> Self {
        Compiler {
            env,
            chunk: Chunk::default(),
            forms: vec![Form::Value(ast)],
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
                Form::Return(_) => return true,
                Form::IfThen(_, _) | Form::IfElse(_, _) => continue,
                _ => return false,
            }
        }
        true
    }

    fn register_local(&mut self, key: &Value) -> Result<()> {
        if let Value::Symbol(symbol) = key {
            let locals = self.locals.last_mut().unwrap();
            locals.insert(*symbol, locals.len().try_into().expect("Too many locals"));
            Ok(())
        } else {
            Err(error_msg("Only symbols can be used as args in fn."))
        }
    }

    fn get_local(&mut self, s: Symbol) -> Option<RegID> {
        self.locals.last().unwrap().get(&s).copied()
    }

    pub fn chunk(self) -> Arc<Chunk> {
        Arc::new(self.chunk)
    }

    pub fn set_argc(&mut self, argc: u8) {
        self.argc = argc;
    }

    fn push(&mut self, val: &Value) -> Result<()> {
        let const_idx = self.get_const_idx(val)?;
        self.emit(Op::Push(const_idx));
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
            Value::Symbol(symbols::DO) => {
                if list.len() < 2 {
                    return Err(error_msg("A do form must contains at least 1 parameter"));
                }
                self.forms.push(Form::Do(list, 1));
            }
            Value::Symbol(symbols::FN) => {
                if list.len() != 3 {
                    return Err(error_msg("A fn form must contains 2 parameters"));
                }
                match &list[1] {
                    Value::List(args) => {
                        // We save the current chunk
                        let chunk = std::mem::take(&mut self.chunk);
                        self.forms.push(Form::Return(chunk));

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
                self.push(&list[1])?;
                self.forms.push(Form::Define);
                self.forms.push(Form::Value(list[2].clone()));
            }
            Value::Symbol(symbols::IF) => {
                if list.len() != 4 {
                    return Err(error_msg("An if form must have 3 parameters"));
                }
                let cond = list[1].clone();
                self.forms.push(Form::IfCond(list));
                self.forms.push(Form::Value(cond));
            }
            _ => {
                self.forms.push(Form::Apply);
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

    pub fn eval_next_in_do(&mut self, list: ZapList, idx: usize) {
        let item = list[idx as usize].clone();
        if (list.len() - 1) > idx {
            self.forms.push(Form::Do(list, idx + 1));
        }
        if idx > 1 {
            self.emit(Op::Pop);
        }
        self.forms.push(Form::Value(item));
    }

    pub fn eval_const(&mut self, val: &Value) -> Result<()> {
        self.push(val)?;
        Ok(())
    }

    pub fn eval_symbol(&mut self, s: Symbol) -> Result<()> {
        if let Some(offset) = self.get_local(s) {
            self.emit(Op::Local(offset));
        } else {
            self.push(&Value::Symbol(s))?;
            self.emit(Op::LookUp);
        }
        Ok(())
    }

    pub fn eval_define(&mut self) -> Result<()> {
        self.emit(Op::Define);
        Ok(())
    }

    pub fn apply(&mut self) -> Result<()> {
        if self.is_last_exp() {
            self.emit(Op::Tailcall(self.argc));
        } else {
            self.emit(Op::Call(self.argc));
        }

        Ok(())
    }

    pub fn eval_then_branch(&mut self, args: ZapList) {
        let branch = args[2].clone();
        self.forms
            .push(Form::IfThen(args, std::mem::take(&mut self.chunk.ops)));
        self.forms.push(Form::Value(branch));
    }

    pub fn eval_else_branch(&mut self, args: ZapList, chunk: Vec<Op>) {
        let branch = args[3].clone();
        self.forms
            .push(Form::IfElse(chunk, std::mem::take(&mut self.chunk.ops)));
        self.forms.push(Form::Value(branch));
    }

    pub fn combine_branches(&mut self, chunk: Vec<Op>, then_branch: Vec<Op>) -> Result<()> {
        let else_branch = std::mem::replace(&mut self.chunk.ops, chunk);

        let then_jump = (1 + then_branch.len())
            .try_into()
            .map_err(|_| error_msg("Then branch jump is too big."))?;
        self.emit(Op::CondJmp(then_jump));
        self.chunk.ops.extend(then_branch);

        let else_jump = else_branch
            .len()
            .try_into()
            .map_err(|_| error_msg("Else branch jump is too big."))?;
        self.emit(Op::Jmp(else_jump));
        self.chunk.ops.extend(else_branch);

        Ok(())
    }

    pub fn wrap_fn(&mut self, mut chunk: Chunk) {
        // Swap the chunks
        std::mem::swap(&mut self.chunk, &mut chunk);
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
            Form::Apply => {
                compiler.apply()?;
            }
            Form::IfCond(args) => {
                // Then branch
                compiler.eval_then_branch(args);
            }
            Form::IfThen(args, chunk) => {
                // Else branch
                compiler.eval_else_branch(args, chunk);
            }
            Form::IfElse(chunk, then_branch) => {
                // Combine the branches in the chunk
                compiler.combine_branches(chunk, then_branch)?;
            }
            Form::Do(list, idx) => {
                compiler.eval_next_in_do(list, idx);
            }
            Form::Define => {
                compiler.eval_define()?;
            }
            Form::Return(chunk) => compiler.wrap_fn(chunk),
        }
    }

    Ok(compiler.chunk())
}
