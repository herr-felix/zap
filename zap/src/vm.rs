use core::ptr;
use std::fmt;
use std::sync::Arc;

use crate::env::Env;
use crate::zap::{error_msg, Result, Symbol, Value};

// Here lives the VM.

#[repr(align(8))]
#[derive(Clone, Copy, PartialEq)]
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
    Return, // Reserved for end of chunk
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Push(const_idx) => write!(f, "PUSH        const({})", const_idx),
            Op::Call(argc) => {
                write!(f, "CALL        argc({})", argc)
            }
            Op::Tailcall(argc) => {
                write!(f, "TAILCALL    argc({})", argc)
            }
            Op::CondJmp(n) => write!(f, "CONDJMP     {}", n),
            Op::Jmp(n) => write!(f, "JMP         {}", n),
            Op::LookUp(id) => write!(f, "LOOKUP      #{}", id),
            Op::Define => write!(f, "DEFINE"),
            Op::Pop => write!(f, "POP"),
            Op::Load(idx) => write!(f, "lOAD        {}", idx),
            Op::AddConst(idx) => write!(f, "ADDCONST    const({})", idx),
            Op::Add => write!(f, "ADD"),
            Op::EqConst(idx) => write!(f, "EQCONST     const({})", idx),
            Op::Eq => write!(f, "EQ"),
            Op::Return => write!(f, "RETURN"),
        }
    }
}

#[derive(Default, Debug)]
pub struct Chunk {
    pub ops: Vec<Op>,
    pub consts: Vec<Value>,
}

impl Chunk {
    #[inline]
    fn get_callframe(&self, ret: usize) -> CallFrame {
        CallFrame {
            pc: self.ops.as_ptr(),
            #[cfg(debug_assertions)]
            start: self.ops.as_ptr(),
            consts: self.consts.as_ptr(),
            ret,
        }
    }
}

struct CallFrame {
    pc: *const Op,
    #[cfg(debug_assertions)]
    start: *const Op,
    consts: *const Value,
    ret: usize,
}

#[repr(align(64))]
struct VmState {
    callframe: CallFrame,
    stack: Vec<Value>,
    calls: Vec<CallFrame>,
}

impl VmState {
    fn new(chunk: &Arc<Chunk>) -> Self {
        VmState {
            callframe: chunk.get_callframe(0),
            calls: Vec::with_capacity(4),
            stack: Vec::with_capacity(8),
        }
    }

    #[inline]
    fn get_next_op(&mut self) -> Op {
        unsafe {
            let pc = self.callframe.pc;
            self.callframe.pc = pc.add(1);
            *pc
        }
    }

    #[inline]
    fn pop_call(&mut self) -> bool {
        if let Some(frame) = self.calls.pop() {
            let tos = self.stack.len() - 1;
            self.stack.swap(self.callframe.ret, tos);
            self.stack.truncate(self.callframe.ret + 1);
            self.callframe = frame;
            true
        } else {
            false
        }
    }

    #[inline]
    fn call(&mut self, argc: usize) -> Result<()> {
        let ret = self.stack.len() - argc;
        match unsafe { &self.stack.get_unchecked(ret) } {
            Value::Func(chunk) => {
                self.calls.push(std::mem::replace(
                    &mut self.callframe,
                    chunk.get_callframe(ret),
                ));
                Ok(())
            }
            Value::FuncNative(f) => {
                let args = unsafe { &self.stack.get_unchecked((ret + 1)..self.stack.len()) };

                let mut output = (f.func)(args)?;
                self.stack.truncate(ret + 1);
                std::mem::swap(self.stack.last_mut().unwrap(), &mut output);
                Ok(())
            }
            _ => Err(error_msg("Cannot call a non-function")),
        }
    }

    #[inline]
    fn tailcall(&mut self, argc: usize) -> Result<()> {
        let args_base = self.stack.len() - argc;
        match unsafe { &self.stack.get_unchecked(args_base) } {
            Value::Func(chunk) => {
                self.callframe = chunk.get_callframe(self.callframe.ret);

                // Move the args
                unsafe {
                    let start = self.stack.as_mut_ptr().add(self.callframe.ret);
                    ptr::swap_nonoverlapping(start, start.add(args_base), argc);
                }
                self.stack.truncate(self.callframe.ret + argc);

                Ok(())
            }
            Value::FuncNative(f) => {
                let args = unsafe { &self.stack.get_unchecked((args_base + 1)..self.stack.len()) };

                let mut output = (f.func)(args)?;
                self.stack.truncate(self.callframe.ret + 1);
                std::mem::swap(self.stack.last_mut().unwrap(), &mut output);
                Ok(())
            }
            _ => Err(error_msg("Cannot call a non-function")),
        }
    }

    #[inline]
    fn push(&mut self, val: Value) {
        self.stack.push(val);
    }

    #[inline]
    fn pop_void(&mut self) {
        self.stack.truncate(self.stack.len() - 1);
    }

    #[inline]
    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap()
    }

    #[inline]
    fn get_top_mut(&mut self) -> *mut Value {
        unsafe { self.stack.as_mut_ptr().add(self.stack.len() - 1) }
    }

    #[inline]
    fn get_const(&mut self, idx: u16) -> &Value {
        unsafe { &*self.callframe.consts.add(idx.into()) }
    }

    #[inline]
    fn jump(&mut self, n: u16) {
        unsafe { self.callframe.pc = self.callframe.pc.add(n as usize) };
    }

    #[inline]
    fn cond_jump(&mut self, n: u16) {
        if !self.pop().is_truthy() {
            self.jump(n);
        }
    }

    #[inline]
    fn lookup<E: Env>(&mut self, id: Symbol, env: &mut E) -> Result<()> {
        let val = env.get_by_id(id)?;
        self.stack.push(val);
        Ok(())
    }

    #[inline]
    fn define<E: Env>(&mut self, env: &mut E) -> Result<()> {
        env.set(
            &self.stack.swap_remove(self.stack.len() - 2),
            self.stack.last().unwrap(),
        )
    }

    #[inline]
    fn push_const(&mut self, idx: u16) {
        let val = self.get_const(idx).clone();
        self.push(val);
    }

    #[inline]
    fn load(&mut self, idx: u8) {
        self.push(
            unsafe {
                self.stack
                    .get_unchecked(self.callframe.ret + (idx as usize) + 1)
            }
            .clone(),
        );
    }

    #[inline]
    fn add_const(&mut self, idx: u16) -> Result<()> {
        unsafe {
            let a = self.get_top_mut();
            let b = self.get_const(idx);
            *a = (&*a + &*b)?
        }
        Ok(())
    }

    #[inline]
    fn add(&mut self) -> Result<()> {
        unsafe {
            let a = self.get_top_mut();
            let b = a.sub(1);
            *b = (&*a + &*b)?
        }
        self.pop_void();
        Ok(())
    }

    #[inline]
    fn eq_const(&mut self, idx: u16) {
        unsafe {
            let a = self.get_top_mut();
            let b = self.get_const(idx);
            *a = Value::Bool(*a == *b);
        }
    }

    #[inline]
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
    let mut vm = VmState::new(&chunk);

    loop {
        let op = vm.get_next_op();
        match op {
            Op::Push(const_idx) => vm.push_const(const_idx),
            Op::Call(argc) => vm.call(argc.into())?,
            Op::Tailcall(argc) => vm.tailcall(argc.into())?,
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
            Op::Return => {
                if !vm.pop_call() {
                    return Ok(vm.pop())
                }
            }
        };

        #[cfg(debug_assertions)]
        #[allow(clippy::format_in_format_args)]
        {
            println!(
                "OP: {:0>5} {:<30} {}",
                unsafe { vm.callframe.pc.offset_from(vm.callframe.start) },
                format!("{:?}", &op),
                format!("STACK: {:?}", &vm.stack)
            );
        }
    }

}
