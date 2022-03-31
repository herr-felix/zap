use core::ptr;
use std::fmt;
use std::sync::Arc;

use crate::env::Env;
use crate::zap::{error_msg, Result, Symbol, Value, ZapFn};

// Here lives the VM.

#[derive(Clone, Copy)]
pub enum Op {
    Push(u16),      // Push a constant on the top of the stack
    Call(u8),       // Call the function at stack[len-argc]
    Tailcall(u8),   // Call the function at stack[len-argc], but truncate the stack to ret
    CondJmp(u16),   // Jump forward n ops if the top of the stack is falsy
    Jmp(u16),       // Jump forward n ops
    LookUp(Symbol), // LookUp the value of a constant and push result
    Define, // Associate the value at the top with the symbol right under it and set the value back at the top
    Pop,    // Pop the top of the stack
    Load(u8), // Push a load on the stack
    AddConst(u16), // Add the element at the top of the stack and a constant and push the result
    Add,    // Add 2 elements at the top of the stack and push the result
    EqConst(u16), // Compare the element at the top of the stack with a constant push true if they're equal and false if they aren't
    Eq, // Compare 2 elements at the top of the stack and push true if they're equal and false if they aren't
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Push(const_idx) => write!(f, "PUSH      const({})", const_idx),
            Op::Call(argc) => {
                write!(f, "CALL      argc({})", argc)
            }
            Op::Tailcall(argc) => {
                write!(f, "TAILCALL  argc({})", argc)
            }
            Op::CondJmp(n) => write!(f, "CONDJMP   {}", n),
            Op::Jmp(n) => write!(f, "JMP       {}", n),
            Op::LookUp(id) => write!(f, "LOOKUP    #{}", id),
            Op::Define => write!(f, "DEFINE"),
            Op::Pop => write!(f, "POP"),
            Op::Load(idx) => write!(f, "lOAD      {}", idx),
            Op::AddConst(idx) => write!(f, "ADDCONST  const({})", idx),
            Op::Add => write!(f, "ADD"),
            Op::EqConst(idx) => write!(f, "EQCONST   const({})", idx),
            Op::Eq => write!(f, "EQ"),
        }
    }
}

#[derive(Default, Debug)]
pub struct Chunk {
    pub ops: Vec<Op>,
    pub consts: Vec<Value>,
}

impl Chunk {
    pub fn get_start(&self) -> *const Op {
        self.ops.as_ptr()
    }

    #[inline(always)]
    pub fn get_end(&self) -> *const Op {
        unsafe { self.ops.as_ptr().add(self.ops.len() - 1) }
    }

    #[inline(always)]
    pub fn get_const(&self, idx: u16) -> &Value {
        unsafe { &*self.consts.as_ptr().add(idx.into()) }
    }
}

struct CallFrame {
    chunk: Arc<Chunk>,
    pc: *const Op,
    ret: usize,
}

struct VmState {
    pc: *const Op,
    end: *const Op,
    ret: usize,
    chunk: Arc<Chunk>,
    calls: Vec<CallFrame>,
    stack: Vec<Value>,
}

impl VmState {
    pub fn new(chunk: Arc<Chunk>) -> Self {
        VmState {
            pc: chunk.get_start(),
            end: chunk.get_end(),
            ret: 0,
            chunk,
            calls: Vec::with_capacity(8),
            stack: Vec::with_capacity(16),
        }
    }

    pub fn get_next_op(&mut self) -> Option<Op> {
        if self.pc > self.end {
            return None;
        }
        unsafe {
            let pc = self.pc;
            self.pc = pc.add(1);
            Some(*pc)
        }
    }

    fn tailcall(&mut self, new_chunk: Arc<Chunk>, argc: usize) {
        self.chunk = new_chunk;
        self.pc = self.chunk.get_start();
        let args_base = self.stack.len() - argc;
        // Move the args
        unsafe {
            let start = self.stack.as_mut_ptr().add(self.ret);
            ptr::swap_nonoverlapping(start, start.add(args_base), argc);
        }
        self.stack.truncate(self.ret + argc);
    }

    fn push_call(&mut self, new_chunk: Arc<Chunk>, argc: usize) {
        // Swap the chunks!
        let old_chunk = std::mem::replace(&mut self.chunk, new_chunk);

        self.calls.push(CallFrame {
            chunk: old_chunk,
            ret: self.ret,
            pc: self.pc,
        });

        self.ret = self.stack.len() - argc;
        self.pc = self.chunk.get_start();
    }

    fn pop_call(&mut self) -> bool {
        if let Some(frame) = self.calls.pop() {
            let tos = self.stack.len() - 1;
            self.stack.swap(self.ret, tos);
            self.stack.truncate(self.ret + 1);
            self.chunk = frame.chunk;
            self.pc = frame.pc;
            self.ret = frame.ret;
            self.end = self.chunk.get_end();
            true
        } else {
            false
        }
    }

    fn call(&mut self, argc: usize, is_tailcall: bool) -> Result<()> {
        if let Value::Func(f) = unsafe { &self.stack.get_unchecked(self.stack.len() - argc) } {
            match f {
                ZapFn::Native(f) => {
                    let args = unsafe {
                        &self
                            .stack
                            .get_unchecked((self.stack.len() - argc + 1)..self.stack.len())
                    };

                    let mut ret = (f.func)(args)?;
                    self.stack.truncate(self.stack.len() - argc + 1);
                    std::mem::swap(self.stack.last_mut().unwrap(), &mut ret);
                }
                ZapFn::Chunk(chunk) => {
                    let new_chunk = chunk.clone();
                    if is_tailcall {
                        self.tailcall(new_chunk, argc);
                    } else {
                        self.push_call(new_chunk, argc);
                    }
                    self.end = self.chunk.get_end();
                }
            }
            Ok(())
        } else {
            Err(error_msg("Cannot call a non-function"))
        }
    }

    #[inline(always)]
    fn push(&mut self, val: Value) {
        self.stack.push(val);
    }

    #[inline(always)]
    fn pop_void(&mut self) {
        self.stack.truncate(self.stack.len() - 1);
    }

    #[inline(always)]
    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    #[inline(always)]
    fn get_top(&mut self) -> *const Value {
        unsafe { self.stack.as_ptr().add(self.stack.len() - 1) }
    }

    #[inline(always)]
    fn get_top_mut(&mut self) -> *mut Value {
        unsafe { self.stack.as_mut_ptr().add(self.stack.len() - 1) }
    }

    #[inline(always)]
    fn jump(&mut self, n: u16) {
        unsafe { self.pc = self.pc.add(n as usize) };
    }

    #[inline(always)]
    fn cond_jump(&mut self, n: u16) {
        if !self.pop().is_truthy() {
            self.jump(n);
        }
    }

    #[inline(always)]
    fn lookup<E: Env>(&mut self, id: Symbol, env: &mut E) -> Result<()> {
        let val = env.get_by_id(id)?;
        self.stack.push(val);
        Ok(())
    }

    #[inline(always)]
    fn define<E: Env>(&mut self, env: &mut E) -> Result<()> {
        env.set(
            &self.stack.swap_remove(self.stack.len() - 2),
            self.stack.last().unwrap(),
        )
    }

    #[inline(always)]
    fn push_const(&mut self, idx: u16) {
        self.push(self.chunk.get_const(idx).clone());
    }

    #[inline(always)]
    fn load(&mut self, idx: u8) {
        self.push(unsafe { self.stack.get_unchecked(self.ret + (idx as usize) + 1) }.clone());
    }

    #[inline(always)]
    fn add_const(&mut self, idx: u16) -> Result<()> {
        unsafe {
            let a = self.get_top_mut();
            let b = self.chunk.get_const(idx);
            *a = (&*a + &*b)?
        }
        Ok(())
    }

    #[inline(always)]
    fn add(&mut self) -> Result<()> {
        unsafe {
            let a = self.get_top_mut();
            let b = a.sub(1);
            *b = (&*a + &*b)?
        }
        self.pop_void();
        Ok(())
    }

    #[inline(always)]
    fn eq_const(&mut self, idx: u16) {
        unsafe {
            let a = self.get_top_mut();
            let b = self.chunk.get_const(idx);
            *a = Value::Bool(*a == *b);
        }
    }

    #[inline(always)]
    fn eq(&mut self) {
        unsafe {
            let a = self.get_top_mut();
            let b = a.sub(1);
            *b = Value::Bool(*a == *b);
        }
        self.pop_void();
    }
}

#[inline(always)]
pub fn run<E: Env>(chunk: Arc<Chunk>, env: &mut E) -> Result<Value> {
    let mut vm = VmState::new(chunk);

    loop {
        if let Some(op) = vm.get_next_op() {
            match op {
                Op::Push(const_idx) => vm.push_const(const_idx),
                Op::Call(argc) => vm.call(argc.into(), false)?,
                Op::Tailcall(argc) => vm.call(argc.into(), true)?,
                Op::CondJmp(n) => vm.cond_jump(n),
                Op::Jmp(n) => vm.jump(n),
                Op::LookUp(id) => vm.lookup(id, env)?,
                Op::Define => vm.define(env)?,
                Op::Pop => {
                    vm.pop_void();
                }
                Op::Load(offset) => vm.load(offset),
                Op::AddConst(const_idx) => vm.add_const(const_idx)?,
                Op::Add => vm.add()?,
                Op::EqConst(const_idx) => vm.eq_const(const_idx),
                Op::Eq => vm.eq(),
            };

            #[cfg(debug_assertions)]
            #[allow(clippy::format_in_format_args)]
            {
                println!(
                    "OP: {:<30} {}",
                    format!("{:?}", &op),
                    format!("STACK: {:?}", &vm.stack)
                );
            }
        } else {
            if !vm.pop_call() {
                break;
            }
        }
    }

    Ok(vm.pop())
}
