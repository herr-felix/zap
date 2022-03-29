use core::ptr;
use std::fmt;
use std::sync::Arc;

use crate::env::Env;
use crate::zap::{error_msg, Result, Value, ZapFn};

// Here lives the VM.

pub type RegID = u8;
pub type Regs = Vec<Value>;

#[derive(Clone, Copy)]
pub enum Op {
    Push(u16),    // Push a constant on the top of the stack
    Call(u8),     // Call the function at stack[len-argc]
    Tailcall(u8), // Call the function at stack[len-argc], but truncate the stack to ret
    CondJmp(u16), // Jump forward n ops if the top of the stack is falsy
    Jmp(u16),     // Jump forward n ops
    LookUp,       // LookUp the value at the top of the stack and push result
    Define, // Associate the value at the top with the symbol right under it and set the value back at the top
    Pop,    // Pop the top of the stack
    Local(u8), // Push a local on the stack
    Add,    // Add 2 elements at the top of the stack and push the result
    Eq, // Compare 2 elements at the top of the stack and push true if they're equal and false if they aren't
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Push(const_idx) => write!(f, "LOAD      const({})", const_idx),
            Op::Call(argc) => {
                write!(f, "CALL      argc({})", argc)
            }
            Op::Tailcall(argc) => {
                write!(f, "TAILCALL  argc({})", argc)
            }
            Op::CondJmp(n) => write!(f, "CONDJMP   {}", n),
            Op::Jmp(n) => write!(f, "JMP       {}", n),
            Op::LookUp => write!(f, "LOOKUP"),
            Op::Define => write!(f, "DEFINE"),
            Op::Pop => write!(f, "POP"),
            Op::Local(idx) => write!(f, "LOCAL     {}", idx),
            Op::Add => write!(f, "ADD"),
            Op::Eq => write!(f, "EQ"),
        }
    }
}

#[derive(Default, Debug)]
pub struct Chunk {
    pub ops: Vec<Op>,
    pub consts: Vec<Value>,
}

struct CallFrame {
    chunk: Arc<Chunk>,
    pc: usize,
    ret: usize,
}

pub struct VM {
    pc: usize,
    ret: usize,
    chunk: Arc<Chunk>,
    calls: Vec<CallFrame>,
    stack: Vec<Value>,
}

impl VM {
    pub fn init() -> Self {
        VM {
            pc: 0,
            ret: 0,
            chunk: Arc::new(Chunk::default()),
            calls: Vec::with_capacity(8),
            stack: Vec::with_capacity(16),
        }
    }

    fn tailcall(&mut self, new_chunk: Arc<Chunk>, argc: usize) {
        self.chunk = new_chunk;
        self.pc = 0;
        let args_base = self.stack.len() - argc;
        // Move the args
        if args_base != self.ret {
            let ptr = self.stack.as_mut_ptr();
            unsafe {
                for offset in 0..argc {
                    ptr::swap(ptr.add(self.ret + offset), ptr.add(args_base + offset));
                }
            }
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
        self.pc = 0;
    }

    fn pop_call(&mut self) -> bool {
        let tos = self.stack.len() - 1;
        self.stack.swap(self.ret, tos);
        self.stack.truncate(self.ret + 1);
        if let Some(frame) = self.calls.pop() {
            self.chunk = frame.chunk;
            self.pc = frame.pc;
            self.ret = frame.ret;
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
    fn jump(&mut self, n: u16) {
        self.pc += n as usize;
    }

    #[inline(always)]
    fn cond_jump(&mut self, n: u16) {
        if !self.pop().is_truthy() {
            self.jump(n);
        }
    }

    #[inline(always)]
    fn lookup<E: Env>(&mut self, env: &mut E) -> Result<()> {
        let tos = self.stack.last_mut().unwrap();
        let mut val = env.get(tos)?;
        std::mem::swap(tos, &mut val);
        Ok(())
    }

    #[inline(always)]
    fn define<E: Env>(&mut self, env: &mut E) -> Result<()> {
        env.set(
            &self.stack.swap_remove(self.stack.len() - 2),
            self.stack.last().unwrap(),
        )?;
        Ok(())
    }

    #[inline(always)]
    fn push_const(&mut self, idx: u16) {
        self.push(self.chunk.consts[idx as usize].clone());
    }

    #[inline(always)]
    fn local(&mut self, idx: u8) {
        self.push(unsafe { self.stack.get_unchecked(self.ret + (idx as usize) + 1) }.clone());
    }

    #[inline(always)]
    pub fn run<E: Env>(&mut self, chunk: Arc<Chunk>, env: &mut E) -> Result<Value> {
        self.pc = 0;
        self.ret = 0;
        self.stack = Vec::with_capacity(8);
        self.chunk = chunk;

        loop {
            if self.chunk.ops.len() > self.pc {
                let op = unsafe { *self.chunk.ops.get_unchecked(self.pc) };
                self.pc += 1;

                #[cfg(debug_assertions)]
                #[allow(clippy::format_in_format_args)]
                {
                    println!(
                        "OP: {:<30} {}",
                        format!("{:?}", &op),
                        format!("STACK: {:?}", &self.stack)
                    );
                }

                match op {
                    Op::Push(const_idx) => self.push_const(const_idx),
                    Op::Call(argc) => self.call(argc.into(), false)?,
                    Op::Tailcall(argc) => self.call(argc.into(), true)?,
                    Op::CondJmp(n) => self.cond_jump(n),
                    Op::Jmp(n) => self.jump(n),
                    Op::LookUp => self.lookup(env)?,
                    Op::Define => self.define(env)?,
                    Op::Pop => {
                        self.pop_void();
                    }
                    Op::Local(offset) => self.local(offset),
                    Op::Add => {
                        let len = self.stack.len();
                        let mut sum = unsafe {
                            self.stack.get_unchecked(len - 1) + self.stack.get_unchecked(len - 2)
                        }?;
                        std::mem::swap(unsafe { self.stack.get_unchecked_mut(len - 2) }, &mut sum);
                        self.pop_void();
                    }
                    Op::Eq => {
                        let len = self.stack.len();
                        let mut res = Value::Bool(unsafe {
                            self.stack.get_unchecked(len - 1) == self.stack.get_unchecked(len - 2)
                        });
                        std::mem::swap(unsafe { self.stack.get_unchecked_mut(len - 2) }, &mut res);
                        self.pop_void();
                    }
                };
            } else if !self.pop_call() {
                break;
            }
        }

        Ok(self.pop())
    }
}
