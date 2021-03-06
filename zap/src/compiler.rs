use crate::env::symbols;
use crate::vm::{Chunk, LocalIndex, Op};
use crate::zap::{error_msg, Result, Symbol, Value, ZapFn, ZapList};
use std::cmp::max;
use std::sync::Arc;

// The compiler takes the expression returned by the reader and return an array of bytecodes
// which can be executed by the VM.

struct Scoping {
    scopes: Vec<(LocalIndex, Vec<Symbol>)>,
    outers: Vec<Vec<Outer>>,
}

impl Default for Scoping {
    fn default() -> Self {
        Scoping {
            scopes: vec![(0, Vec::new())],
            outers: vec![Vec::new()],
        }
    }
}

impl Scoping {
    pub fn push_local(&mut self, symbol: Symbol) -> Result<LocalIndex> {
        // Add a symbol in the scope
        let (max_len, mut locals) = self.scopes.pop().unwrap();
        locals.push(symbol);
        let len = locals
            .len()
            .try_into()
            .map_err(|_| error_msg("Too many locals in scope!"))?;
        self.scopes.push((max(max_len, len), locals));
        Ok(len - 1)
    }

    pub fn pop_locals(&mut self, count: usize) {
        // Pop symbols from the scope
        let (_, scope) = self.scopes.last_mut().unwrap();
        let new_len = scope.len() - count;
        scope.truncate(new_len);
    }

    pub fn get_local(&mut self, s: Symbol) -> Option<usize> {
        // Look if this symbol is in the current scope
        let (_, scope) = self.scopes.last_mut().unwrap();
        for (offset, local) in scope.iter().enumerate().rev() {
            if *local == s {
                return Some(offset);
            }
        }
        None
    }

    pub fn get_outer(&mut self, s: Symbol) -> Option<(usize, usize)> {
        // Look if this symbol is in the current scope
        for (level, (_, scope)) in self.scopes.iter().enumerate().rev() {
            for (position, symbol) in scope.iter().enumerate().rev() {
                if *symbol == s {
                    return Some((level, position));
                }
            }
        }
        None
    }

    pub fn push_outer(&mut self, level: usize, position: usize, dest: LocalIndex) {
        let outer = Outer {
            level,
            position,
            dest,
        };
        self.outers.last_mut().unwrap().push(outer);
    }

    pub fn push(&mut self) {
        self.scopes.push((0, Vec::new()));
        self.outers.push(Vec::new());
    }

    pub fn pop(&mut self) -> (usize, Vec<Outer>) {
        let (size, _) = self.scopes.pop().unwrap();
        let outers = self.outers.pop().unwrap();
        (size.into(), outers)
    }
}

#[derive(Debug)]
pub struct Outer {
    pub level: usize,
    pub position: usize,
    pub dest: LocalIndex,
}

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
    AddMany(ZapList, usize),
    Add,
    Equal,
    EqualConst(u16),
    Let(usize),
    Binding(Symbol),
    Quoting,
}

struct Compiler {
    chunk: Chunk,
    forms: Vec<Form>,
    scopes: Scoping,
    argc: u8,
    quoting: bool,
}

impl Compiler {
    pub fn init(ast: Value) -> Self {
        Compiler {
            chunk: Chunk::default(),
            forms: vec![Form::Value(ast)],
            scopes: Scoping::default(),
            argc: 0,
            quoting: false,
        }
    }

    pub fn get_form(&mut self) -> Option<Form> {
        self.forms.pop()
    }

    fn is_last_exp(&self) -> bool {
        for form in self.forms.iter().rev() {
            match form {
                Form::IfThen(_, _) | Form::IfElse(_, _) | Form::Let(_) => continue,
                Form::Return(_) => return true,
                _ => return false,
            }
        }
        true
    }

    pub fn register_binding(&mut self, symbol: Symbol) -> Result<()> {
        let idx = self.scopes.push_local(symbol)?;
        self.emit(Op::Store(idx));
        Ok(())
    }

    pub fn chunk(mut self) -> Arc<Chunk> {
        self.emit(Op::Return);
        let (count, _) = self.scopes.pop();
        self.chunk.scope_size = count;
        self.chunk.ops.shrink_to_fit();
        self.chunk.consts.shrink_to_fit();
        Arc::new(self.chunk)
    }

    pub fn set_argc(&mut self, argc: u8) {
        self.argc = argc - 1;
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

        // In quoting
        if self.quoting {
            self.forms.push(Form::List(list.clone(), 0));
            self.forms.push(Form::Value(list[0].clone()));
            return Ok(());
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

                // Get into another scope
                self.scopes.push();

                match &list[1] {
                    Value::List(args) => {
                        // We save the current chunk
                        let parent_chunk = std::mem::take(&mut self.chunk);
                        self.forms.push(Form::Return(parent_chunk));

                        self.chunk.arity = args.len().try_into().unwrap();

                        // Set all the params in the locals.
                        for arg in args.iter() {
                            if let Value::Symbol(symbol) = arg {
                                self.scopes.push_local(*symbol)?;
                            } else {
                                return Err(error_msg("Only symbols can be used as args in fn."));
                            }
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
                    return Err(error_msg("A def form must have 2 parameters"));
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
            Value::Symbol(symbols::LET) => {
                if list.len() != 3 {
                    return Err(error_msg("A let form must have 2 parameters"));
                }

                if let Value::List(bindings) = &list[1] {
                    // Check for even number of bindings
                    if bindings.len() % 2 == 1 {
                        return Err(error_msg("Bindings must have an even number of bindings"));
                    }
                    self.forms.push(Form::Let(bindings.len() / 2));
                    self.forms.push(Form::Value(list[2].clone()));

                    for pair in bindings.rchunks(2) {
                        if let Value::Symbol(s) = pair[0] {
                            self.forms.push(Form::Binding(s));
                            self.forms.push(Form::Value(pair[1].clone()));
                        } else {
                            return Err(error_msg(
                                "A binding must consist of a symbol and an expression",
                            ));
                        }
                    }
                } else {
                    return Err(error_msg("A let form must have a list of bindings"));
                }
            }
            Value::Symbol(symbols::EQUAL) => {
                if list.len() != 3 {
                    return Err(error_msg("A = form must have 2 parameters"));
                }

                if is_const(&list[1]) == is_const(&list[2]) {
                    // Compile time compare on constants
                    self.push(&Value::Bool(list[1] == list[2]))?;
                } else if is_const(&list[1]) {
                    let idx = self.get_const_idx(&list[1].clone())?;
                    self.forms.push(Form::EqualConst(idx));
                    self.forms.push(Form::Value(list[2].clone()));
                } else if is_const(&list[2]) {
                    let idx = self.get_const_idx(&list[2].clone())?;
                    self.forms.push(Form::EqualConst(idx));
                    self.forms.push(Form::Value(list[1].clone()));
                } else {
                    self.forms.push(Form::Equal);
                    self.forms.push(Form::Value(list[1].clone()));
                    self.forms.push(Form::Value(list[2].clone()));
                }
            }
            Value::Symbol(symbols::PLUS) => {
                match list.len() {
                    1 => {
                        // Push 0 on the stack
                        let const_idx = self.get_const_idx(&Value::Number(0.0))?;
                        self.emit(Op::Push(const_idx));
                    }
                    2 => {
                        self.forms.push(Form::Value(list[1].clone()));
                    }
                    _ => {
                        self.forms.push(Form::AddMany(list, 1));
                    }
                }
            }
            Value::Symbol(symbols::QUOTE) => {
                if list.len() != 2 {
                    return Err(error_msg("'quote' require only 1 value"));
                }

                self.push(&list[1])?;
            }
            Value::Symbol(symbols::QUASIQUOTE) => {
                if list.len() != 2 {
                    return Err(error_msg("'quasiquote' require only 1 value"));
                }

                self.quoting = true;
                self.forms.push(Form::Quoting);
                self.forms.push(Form::Value(list[1].clone()));
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
        if let Some(offset) = self.scopes.get_local(s) {
            self.emit(Op::Load(offset.try_into().unwrap()));
        } else if let Some((level, position)) = self.scopes.get_outer(s) {
            let dest = self.scopes.push_local(s)?;
            self.scopes.push_outer(level, position, dest);
            self.emit(Op::Load(dest));
        } else {
            self.emit(Op::LookUp(s));
        }
        Ok(())
    }

    pub fn eval_define(&mut self) {
        self.emit(Op::Define);
    }

    pub fn apply(&mut self) {
        if self.is_last_exp() {
            self.emit(Op::Tailcall(self.argc));
        } else {
            self.emit(Op::Call(self.argc));
        }
    }

    pub fn eval_then_branch(&mut self, args: ZapList) {
        let branch = args[2].clone();
        self.forms
            .push(Form::IfThen(args, std::mem::take(&mut self.chunk.ops)));
        self.forms.push(Form::Value(branch));
    }

    pub fn eval_else_branch(&mut self, args: &ZapList, chunk: Vec<Op>) {
        let branch = args[3].clone();
        self.forms
            .push(Form::IfElse(chunk, std::mem::take(&mut self.chunk.ops)));
        self.forms.push(Form::Value(branch));
    }

    pub fn combine_branches(&mut self, chunk: Vec<Op>, then_branch: Vec<Op>) -> Result<()> {
        let else_branch = std::mem::replace(&mut self.chunk.ops, chunk);

        let then_jump = (then_branch.len() + 1)
            .try_into()
            .map_err(|_| error_msg("Then branch jump is too big."))?;
        self.emit(Op::CondJmp(then_jump));
        self.chunk.ops.extend(then_branch);

        let else_jump = else_branch
            .len()
            .try_into()
            .map_err(|_| error_msg("Else branch jump is too big."))?;
        if self.is_last_exp() {
            self.emit(Op::Return);
        } else {
            self.emit(Op::Jmp(else_jump));
        }
        self.chunk.ops.extend(else_branch);

        Ok(())
    }

    pub fn eval_next_in_add(&mut self, list: &ZapList, idx: usize) -> Result<()> {
        if idx == 1 {
            self.forms.push(Form::AddMany(list.clone(), idx + 1));
            self.forms.push(Form::Value(list[idx].clone()));
        } else if list.len() > idx {
            self.forms.push(Form::AddMany(list.clone(), idx + 1));
            if is_const(&list[idx]) {
                // It's a constant
                let const_idx = self.get_const_idx(&list[idx])?;
                self.emit(Op::AddConst(const_idx));
            } else {
                self.forms.push(Form::Add);
                self.forms.push(Form::Value(list[idx].clone()));
            }
        }
        Ok(())
    }

    pub fn eval_add(&mut self) {
        self.emit(Op::Add);
    }

    pub fn eval_equal(&mut self) {
        self.emit(Op::Eq);
    }

    pub fn eval_equal_const(&mut self, idx: u16) {
        self.emit(Op::EqConst(idx));
    }

    pub fn wrap_fn(&mut self, mut chunk: Chunk) -> Result<()> {
        #[cfg(debug_assertions)]
        dbg!(&self.chunk);

        self.emit(Op::Return);

        let (size, outers) = self.scopes.pop();
        self.chunk.scope_size = size;

        // Swap the chunks
        std::mem::swap(&mut self.chunk, &mut chunk);

        if outers.is_empty() {
            self.push(&ZapFn::new(size, chunk))?;
        } else {
            self.push(&ZapFn::new_closure(outers, chunk))?;
            self.emit(Op::Closure);
        }

        Ok(())
    }
}

pub fn compile(ast: Value) -> Result<Arc<Chunk>> {
    let mut compiler = Compiler::init(ast);

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
                compiler.apply();
            }
            Form::IfCond(args) => {
                // Then branch
                compiler.eval_then_branch(args);
            }
            Form::IfThen(args, chunk) => {
                // Else branch
                compiler.eval_else_branch(&args, chunk);
            }
            Form::IfElse(chunk, then_branch) => {
                // Combine the branches in the chunk
                compiler.combine_branches(chunk, then_branch)?;
            }
            Form::AddMany(list, idx) => {
                compiler.eval_next_in_add(&list, idx)?;
            }
            Form::Add => {
                compiler.eval_add();
            }
            Form::EqualConst(idx) => {
                compiler.eval_equal_const(idx);
            }
            Form::Equal => {
                compiler.eval_equal();
            }
            Form::Do(list, idx) => {
                compiler.eval_next_in_do(list, idx);
            }
            Form::Define => {
                compiler.eval_define();
            }
            Form::Return(chunk) => compiler.wrap_fn(chunk)?,
            Form::Let(locals_count) => {
                compiler.scopes.pop_locals(locals_count);
            }
            Form::Binding(symbol) => {
                compiler.register_binding(symbol)?;
            }
            Form::Quoting => {
                // TODO
            }
        }
    }

    Ok(compiler.chunk())
}

fn is_const(val: &Value) -> bool {
    !matches!(val, Value::List(_) | Value::Symbol(_))
}
