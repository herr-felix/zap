use std::fmt;
use std::sync::Arc;

use crate::env::Env;
use crate::zap::{error_msg, Result, Value, ZapFn};

// Here lives the VM.

pub type RegID = u8;
pub type Regs = Vec<Value>;

#[derive(Clone)]
pub enum Op {
    Move { dst: RegID, src: RegID },     // Copy the content of src to dst
    Load { dst: RegID, const_idx: u16 }, // Load the literal Val into resgister reg
    Add { a: RegID, b: RegID, dst: RegID }, // Add r(a) with r(b) and put the result in r(dst)
    Call { dst: u8, start: u8, argc: u8 }, // Call the function at r(0) with argc arguments
    CondJmp { reg: RegID, n: u16 },      // Jump forward n ops if r(reg) is truty
    Jmp(u16),                            // Jump forward n ops
    LookUp(RegID),                       // LookUp r(id) in the env and puts the results in r(id)
    Define { key: RegID, dst: RegID }, // Associate the value r(dst) with the key r(key) in the env
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Move { dst, src } => write!(f, "MOVE    r({}) <- r({})", dst, src),
            Op::Load { dst, const_idx } => write!(f, "LOAD    r({}) const({})", dst, const_idx),
            Op::Add { a, b, dst } => write!(f, "ADD     r({}) = r({}) + r({})", dst, a, b),
            Op::Call { dst, start, argc } => {
                write!(f, "CALL    r({}) = r({})..{}", dst, start, argc)
            }
            Op::CondJmp { reg, n } => write!(f, "CONDJMP r({}) {}", reg, n),
            Op::Jmp(n) => write!(f, "JMP     {}", n),
            Op::LookUp(reg) => write!(f, "LOOKUP  r({})", reg),
            Op::Define { key, dst } => write!(f, "DEFINE  r({}) r({})", key, dst),
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
    regs: Regs,
    dst: u8,
}

pub struct VM {
    pc: usize,
    chunk: Arc<Chunk>,
    calls: Vec<CallFrame>,
    regs: Regs,
}

impl VM {
    pub fn init() -> Self {
        VM {
            pc: 0,
            chunk: Arc::new(Chunk::default()),
            calls: Vec::with_capacity(8),
            regs: vec![Value::Nil; 256],
        }
    }

    #[inline(always)]
    fn get_next_op(&mut self) -> Option<Op> {
        self.pc += 1;
        self.chunk.ops.get(self.pc - 1).cloned()
    }

    fn push_call(&mut self, new_chunk: Arc<Chunk>, dst: u8) {
        let chunk = std::mem::replace(&mut self.chunk, new_chunk);
        self.calls.push(CallFrame {
            dst,
            chunk,
            regs: self.regs.clone(),
            pc: self.pc,
        });
        self.pc = 0;
    }

    fn pop_call(&mut self) -> bool {
        if let Some(frame) = self.calls.pop() {
            let ret = self.regs[0].clone();
            self.pc = frame.pc;
            self.chunk = frame.chunk;
            self.regs = frame.regs;
            self.set_reg(frame.dst, ret);
            true
        } else {
            false
        }
    }

    #[inline(always)]
    fn set_reg(&mut self, idx: RegID, val: Value) {
        self.regs[idx as usize] = val;
    }

    #[inline(always)]
    fn reg(&self, idx: RegID) -> Value {
        self.regs[idx as usize].clone()
    }

    fn call(&mut self, start: u8, argc: u8, dst: u8) -> Result<()> {
        // Set the chunk in reg(0) as current chunk
        if let Value::Func(f) = self.reg(start) {
            match f {
                ZapFn::Native(f) => {
                    let args = &self.regs[(start as usize)..=(start as usize + argc as usize)];
                    let ret = (f.func)(args)?;
                    self.set_reg(dst, ret);
                }
                ZapFn::Chunk(chunk) => {
                    self.push_call(chunk, dst);
                    // Move the args at the begining
                    for offset in 0..(argc as usize) {
                        self.regs.swap((start as usize) + offset, offset);
                    }
                }
            }
            Ok(())
        } else {
            Err(error_msg("Cannot call a non-function"))
        }
    }

    #[inline(always)]
    fn jump(&mut self, n: u16) {
        self.pc += n as usize;
    }

    #[inline(always)]
    fn load_const(&mut self, dst: u8, idx: u16) {
        self.set_reg(dst, self.chunk.consts[idx as usize].clone());
    }

    pub fn run<E: Env>(&mut self, chunk: Arc<Chunk>, env: &mut E) -> Result<Value> {
        self.pc = 0;
        self.chunk = chunk;

        #[cfg(debug_assertions)]
        dbg!(&self.chunk);

        loop {
            if let Some(op) = self.get_next_op() {
                match op {
                    Op::Move { dst, src } => {
                        self.set_reg(dst, self.regs[src as usize].clone());
                    }
                    Op::Load { dst, const_idx } => self.load_const(dst, const_idx),
                    Op::Add { a, b, dst } => self.set_reg(dst, (self.reg(a) + self.reg(b))?),
                    Op::Call { dst, start, argc } => self.call(start, argc, dst)?,
                    Op::CondJmp { reg, n } => {
                        if self.reg(reg).is_truthy() {
                            self.jump(n)
                        }
                    }
                    Op::Jmp(n) => self.jump(n),
                    Op::LookUp(reg) => self.set_reg(reg, env.get(&self.regs[reg as usize])?),
                    Op::Define { key, dst } => {
                        env.set(&self.regs[key as usize], &self.regs[dst as usize])?
                    }
                }
            } else if !self.pop_call() {
                break;
            }
        }

        Ok(self.regs[0].clone())
    }
}
