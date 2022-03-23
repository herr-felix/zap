use std::fmt;
use std::sync::Arc;

use crate::env::Env;
use crate::zap::{error_msg, Result, Value, ZapFn};

// Here lives the VM.

pub type RegID = u8;

#[derive(Clone)]
pub enum Op {
    Move { dst: RegID, src: RegID }, // Copy the content of src to dst
    Set { dst: RegID, val: Value },  // Load the literal Val into resgister reg
    Add { a: RegID, b: RegID, dst: RegID }, // Add reg(a) with r(b) and put the result in reg(dst)
    Push { val: Value },             // Push val on the stack
    PushRet,                         // Push r(0) on the stack
    Pop { dst: RegID },              // Pop a value from the stack into a register
    Call { argc: u8 },               // Call the function at reg(0) with argc arguments
    CondJmp(usize),                  // Jump forward n ops if reg(0) is truty
    Jmp(usize),                      // Jump forward n ops
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Move { dst, src } => write!(f, "MOVE    {} {}", dst, src),
            Op::Set { dst, val } => write!(f, "SET     {} {}", dst, val),
            Op::Add { a, b, dst } => write!(f, "ADD     {} {} {}", dst, a, b),
            Op::Push { val } => write!(f, "PUSH    {}", val),
            Op::PushRet => write!(f, "PUSHRET"),
            Op::Pop { dst } => write!(f, "POP     {}", dst),
            Op::Call { argc } => write!(f, "CALL    {}", argc),
            Op::CondJmp(n) => write!(f, "CONDJMP {}", n),
            Op::Jmp(n) => write!(f, "JMP     {}", n),
        }
    }
}

pub type Chunk = Arc<Vec<Op>>;

pub struct VM {
    pc: usize,
    chunk: Chunk,
    stack: Vec<Value>,
    call_stack: Vec<(Chunk, usize)>,
    regs: [Value; 256],
}

impl VM {
    pub fn init() -> Self {
        VM {
            pc: 0,
            chunk: Arc::new(Vec::new()),
            stack: Vec::with_capacity(32),
            call_stack: Vec::with_capacity(32),
            regs: [(); 256].map(|_| Value::default()),
        }
    }

    #[inline(always)]
    fn get_next_op(&mut self) -> Option<Op> {
        self.pc += 1;
        self.chunk.get(self.pc - 1).cloned()
    }

    fn pop_call(&mut self) -> bool {
        if let Some((chunk, pc)) = self.call_stack.pop() {
            self.pc = pc;
            self.chunk = chunk;
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

    #[inline(always)]
    fn pop_to_reg(&mut self, reg: RegID) -> Result<()> {
        if let Some(val) = self.stack.pop() {
            self.set_reg(reg, val);
            Ok(())
        } else {
            Err(error_msg("Pop to reg: Stack is empty."))
        }
    }

    #[inline(always)]
    fn push_ret(&mut self) {
        let val = std::mem::take(&mut self.regs[0]);
        self.stack.push(val);
    }

    fn call(&mut self, argc: u8) -> Result<()> {
        // Set the chunk in reg(0) as current chunk
        if let Value::Func(f) = self.reg(0) {
            match f {
                ZapFn::Native(_, native) => {
                    let args = &self.regs[1..=(argc as usize)];
                    let ret = native(args)?;
                    self.set_reg(0, ret);
                }
                ZapFn::Chunk(chunk) => {
                    // Push the current chunk on the summary
                    let parent_chunk = std::mem::replace(&mut self.chunk, chunk);
                    self.call_stack.push((parent_chunk, self.pc));
                    self.pc = 0;
                }
            }
            Ok(())
        } else {
            Err(error_msg("Cannot call a non-function"))
        }
    }

    #[inline(always)]
    fn jump(&mut self, n: usize) {
        self.pc += n;
    }

    pub fn run<E: Env>(&mut self, chunk: Chunk, _env: &mut E) -> Result<Value> {
        self.pc = 0;
        self.chunk = chunk;

        #[cfg(debug_assertions)]
        dbg!(&self.chunk);

        loop {
            if let Some(op) = self.get_next_op() {
                match op {
                    Op::Move { dst, src } => {
                        self.regs[dst as usize] = self.regs[src as usize].clone();
                    }
                    Op::Set { dst, val } => self.set_reg(dst, val),
                    Op::Add { a, b, dst } => self.set_reg(dst, (self.reg(a) + self.reg(b))?),
                    Op::Push { val } => self.stack.push(val),
                    Op::PushRet => self.push_ret(),
                    Op::Pop { dst } => self.pop_to_reg(dst)?,
                    Op::Call { argc } => self.call(argc)?,
                    Op::CondJmp(n) => {
                        if self.reg(0).is_truthy() {
                            self.jump(n)
                        }
                    }
                    Op::Jmp(n) => self.jump(n),
                }
            } else if !self.pop_call() {
                break;
            }
        }

        Ok(self.regs[0].clone())
    }
}
