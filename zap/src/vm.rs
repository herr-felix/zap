use std::fmt;
use std::rc::Rc;

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
        }
    }
}

pub type Chunk = Rc<Vec<Op>>;

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
            chunk: Rc::new(Vec::new()),
            stack: Vec::with_capacity(32),
            call_stack: Vec::with_capacity(32),
            regs: [(); 256].map(|_| Value::default()),
        }
    }

    fn get_next_op(&mut self) -> Option<Op> {
        self.pc += 1;
        // Check if we are at the end of the current chunk. If so, pop a chunk off the stack
        // and set it back as the current chunk. If the stack is empty, we are done running.
        if self.chunk.len() < self.pc {
            if let Some((chunk, pc)) = self.call_stack.pop() {
                self.pc = pc;
                self.chunk = chunk;
            } else {
                return None;
            }
        }
        Some(self.chunk[self.pc - 1].clone())
    }

    fn set_reg(&mut self, idx: RegID, val: Value) {
        self.regs[idx as usize] = val;
    }

    fn reg(&self, idx: RegID) -> Value {
        self.regs[idx as usize].clone()
    }

    fn pop_to_reg(&mut self, reg: RegID) -> Result<()> {
        if let Some(val) = self.stack.pop() {
            self.set_reg(reg, val);
            Ok(())
        } else {
            Err(error_msg("Pop to reg: Stack is empty."))
        }
    }

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

    pub fn run<E: Env>(&mut self, chunk: Chunk, _env: &mut E) -> Result<Value> {
        self.pc = 0;
        self.chunk = chunk;
        dbg!(&self.chunk);

        while let Some(op) = self.get_next_op() {
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
            }
        }

        Ok(self.regs[0].clone())
    }
}
