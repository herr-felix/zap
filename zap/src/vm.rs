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
    Call { start: u8, argc: u8 }, // Call the function at r(0) with argc arguments
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
            Op::Call { start, argc } => {
                write!(f, "CALL    r({}) = r({})..{}", start, start, argc)
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
    pub used_regs: RegID,
}

struct CallFrame {
    chunk: Arc<Chunk>,
    pc: usize,
    saved_regs: Regs,
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
            regs: Vec::new(),
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

    fn tailcall(&mut self, new_chunk: Arc<Chunk>, start: usize, argc: usize) {
        for i in start..argc {
            self.regs.swap(i, i+start);
        }
        self.regs.resize_with(new_chunk.used_regs as usize, Default::default);
        self.chunk = new_chunk;
        self.pc = 0;
    }

    fn push_call(&mut self, new_chunk: Arc<Chunk>, start: u8, argc: usize) {

        #[cfg(debug_assertions)]
        println!("SAVING: {:?}", &self.regs);

        // Create the new register and but the args at the start
        let mut saved_regs = Vec::with_capacity(self.chunk.used_regs as usize);
        saved_regs.fill(Value::Nil);

        std::mem::swap(&mut saved_regs, &mut self.regs); // SWAP!

        let start_idx: usize = start.into();
        for offset in 0..argc {
            std::mem::swap(&mut saved_regs[offset], &mut self.regs[start_idx + offset]);
        }

        // Swap the chunks!
        let old_chunk = std::mem::replace(&mut self.chunk, new_chunk);

        self.calls.push(CallFrame {
            dst: start,
            chunk: old_chunk,
            saved_regs,
            pc: self.pc,
        });

        self.pc = 0;
    }

    fn pop_call(&mut self) -> bool {
        if let Some(frame) = self.calls.pop() {
            let ret = self.regs[0].clone();

            #[cfg(debug_assertions)]
            println!("RETURN: {:?}", &self.regs);

            self.regs = frame.saved_regs;
            self.chunk = frame.chunk;
            self.pc = frame.pc;
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

    fn call(&mut self, start: u8, argc: u8, is_tailcall: bool) -> Result<()> {
        // Set the chunk in reg(0) as current chunk
        if let Value::Func(f) = self.reg(start) {
            match f {
                ZapFn::Native(f) => {
                    let args = &self.regs[((start + 1) as usize)..(start as usize + argc as usize)];

                    #[cfg(debug_assertions)]
                    dbg!(&args);

                    let ret = (f.func)(args)?;
                    self.set_reg(start, ret);
                }
                ZapFn::Chunk(new_chunk) => {
                    if is_tailcall {
                        self.tailcall(new_chunk, start.into(), argc.into());
                    } else {
                        self.push_call(new_chunk, start, argc.into());
                    }
                    #[cfg(debug_assertions)]
                    dbg!("{:?}", &self.chunk.consts);
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
    fn cond_jump(&mut self, reg: RegID, n: u16) {
        if self.reg(reg).is_truthy() {
            self.jump(n)
        }
    }

    #[inline(always)]
    fn lookup<E: Env>(&mut self, reg: RegID, env: &mut E) -> Result<()> {
        self.set_reg(reg, env.get(&self.regs[reg as usize])?);
        Ok(())
    }

    #[inline(always)]
    fn define<E: Env>(&mut self, key: RegID, dst: RegID, env: &mut E) -> Result<()> {
        env.set(&self.regs[key as usize], &self.regs[dst as usize])?;
        Ok(())
    }

    #[inline(always)]
    fn load_const(&mut self, dst: u8, idx: u16) {
        self.set_reg(dst, self.chunk.consts[idx as usize].clone());
    }

    #[inline(always)]
    fn move_op(&mut self, dst: RegID, src: RegID) {
        self.set_reg(dst, self.regs[src as usize].clone());
    }

    pub fn run<E: Env>(&mut self, chunk: Arc<Chunk>, env: &mut E) -> Result<Value> {
        self.pc = 0;
        self.regs.resize_with(chunk.used_regs.into(), Default::default);
        self.chunk = chunk;

        #[cfg(debug_assertions)]
        dbg!(&self.chunk.consts);

        loop {
            if let Some(op) = self.get_next_op() {
                #[cfg(debug_assertions)]
                println!("OP: {:<35} {}", format!("{:?}", &op), format!("REGS: {:?}", &self.regs));

                match op {
                    Op::Move { dst, src } => self.move_op(dst, src),
                    Op::Load { dst, const_idx } => self.load_const(dst, const_idx),
                    Op::Add { a, b, dst } => self.set_reg(dst, (self.reg(a) + self.reg(b))?),
                    Op::Call { start, argc } => self.call(start, argc, false)?,
                    Op::CondJmp { reg, n } => self.cond_jump(reg, n),
                    Op::Jmp(n) => self.jump(n),
                    Op::LookUp(reg) => self.lookup(reg, env)?,
                    Op::Define { key, dst } => self.define(key, dst, env)?,
                }
            } else if !self.pop_call() {
                break;
            }
        }

        #[cfg(debug_assertions)]
        println!("FINAL: {:?}", &self.regs);

        Ok(self.regs[0].clone())
    }
}
