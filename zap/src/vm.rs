use std::fmt;
use std::sync::Arc;

use crate::env::Env;
use crate::zap::{error_msg, Result, Value, ZapFn};

// Here lives the VM.

pub type RegID = u8;
pub type Regs = Vec<Value>;

#[derive(Clone)]
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
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Push(const_idx) => write!(f, "LOAD    const({})", const_idx),
            Op::Call(argc) => {
                write!(f, "CALL    {}", argc)
            }
            Op::Tailcall(argc) => {
                write!(f, "TAILCALL     {}", argc)
            }
            Op::CondJmp(n) => write!(f, "CONDJMP {}", n),
            Op::Jmp(n) => write!(f, "JMP     {}", n),
            Op::LookUp => write!(f, "LOOKUP"),
            Op::Define => write!(f, "DEFINE"),
            Op::Pop => write!(f, "POP"),
            Op::Local(idx) => write!(f, "LOCAL     {}", idx),
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

    #[inline(always)]
    fn get_next_op(&mut self) -> Option<Op> {
        self.pc += 1;
        if self.chunk.ops.len() >= self.pc {
            Some(self.chunk.ops[self.pc - 1].clone())
        } else {
            None
        }
    }

    fn tailcall(&mut self, new_chunk: Arc<Chunk>, argc: usize) {
        self.chunk = new_chunk;
        self.pc = 0;
        let args_base = self.stack.len() - argc;
        // Move the args
        if args_base != self.ret {
            for offset in 0..argc {
                self.stack.swap(self.ret + offset, args_base + offset)
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

        self.pc = 0;
    }

    fn pop_call(&mut self) -> bool {
        if let Some(frame) = self.calls.pop() {
            self.stack.truncate(self.ret);
            self.chunk = frame.chunk;
            self.pc = frame.pc;
            self.ret = frame.ret;
            true
        } else {
            false
        }
    }

    fn call(&mut self, argc: usize, is_tailcall: bool) -> Result<()> {
        if let Value::Func(f) = &self.stack[self.stack.len() - argc] {
            match f {
                ZapFn::Native(f) => {
                    let args = &self.stack[(self.stack.len() - argc + 1)..self.stack.len()];

                    #[cfg(debug_assertions)]
                    dbg!(&args);

                    let ret = (f.func)(args)?;
                    self.stack.truncate(self.stack.len() - argc);
                    self.push(ret);
                }
                ZapFn::Chunk(chunk) => {
                    let new_chunk = chunk.clone();
                    if is_tailcall {
                        self.tailcall(new_chunk, argc);
                    } else {
                        self.push_call(new_chunk, argc);
                    }
                    #[cfg(debug_assertions)]
                    dbg!("{:?}", &self.chunk);
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
            self.jump(n)
        }
    }

    #[inline(always)]
    fn lookup<E: Env>(&mut self, env: &mut E) -> Result<()> {
        let key = &self.pop();
        self.push(env.get(key)?);
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
        self.push(self.stack[self.ret + (idx as usize) + 1].clone());
    }

    pub fn run<E: Env>(&mut self, chunk: Arc<Chunk>, env: &mut E) -> Result<Value> {
        self.pc = 0;
        self.ret = 0;
        self.stack = Vec::with_capacity(8);
        self.chunk = chunk;

        #[cfg(debug_assertions)]
        dbg!(&self.chunk.consts);

        loop {
            if let Some(op) = self.get_next_op() {
                #[cfg(debug_assertions)]
                println!(
                    "OP: {:<30} {}",
                    format!("{:?}", &op),
                    format!("STACK: {:?}", &self.stack)
                );

                match op {
                    Op::Push(const_idx) => self.push_const(const_idx),
                    Op::Call(argc) => self.call(argc.into(), false)?,
                    Op::Tailcall(argc) => self.call(argc.into(), true)?,
                    Op::CondJmp(n) => self.cond_jump(n),
                    Op::Jmp(n) => self.jump(n),
                    Op::LookUp => self.lookup(env)?,
                    Op::Define => self.define(env)?,
                    Op::Pop => {
                        self.pop();
                    }
                    Op::Local(offset) => self.local(offset),
                };
            } else if !self.pop_call() {
                break;
            }
        }

        Ok(self.pop())
    }
}
