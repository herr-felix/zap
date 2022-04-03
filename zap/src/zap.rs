use std::ptr;
use std::sync::Arc;

pub use smartstring::alias::String;

use crate::compiler::Outer;
use crate::env::Env;
use crate::vm::{CallFrame, Chunk};

pub type Symbol = u32;

pub type ZapList = Arc<Vec<Value>>;
pub type Result<T> = std::result::Result<T, ZapErr>;

#[derive(Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Symbol(Symbol),
    Str(String),
    List(ZapList),
    FuncNative(Arc<ZapFnNative>),
    Func(Arc<ZapFn>),
    Closure(Arc<Closure>),
}

impl Value {
    pub fn to_string<E: Env>(&self, _env: &mut E) -> std::string::String {
        match self {
            Value::Func(_) => "Func<>".to_string(),
            x => format!("{}", x),
        }
    }

    pub fn new_list(list: Vec<Value>) -> ZapList {
        Arc::new(list)
    }

    #[inline(always)]
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl core::ops::Add for &Value {
    type Output = Result<Value>;

    #[inline(always)]
    fn add(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (a, b) => Err(error_msg(format!("Can't add {} + {}", a, b).as_str())),
        }
    }
}

impl core::ops::Sub for Value {
    type Output = Result<Value>;

    #[inline(always)]
    fn sub(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            (a, b) => Err(error_msg(format!("Can't substract {} - {}", a, b).as_str())),
        }
    }
}

impl core::ops::Mul for Value {
    type Output = Result<Value>;

    #[inline(always)]
    fn mul(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            (a, b) => Err(error_msg(format!("Can't multiply {} - {}", a, b).as_str())),
        }
    }
}

impl PartialEq for Value {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::List(a), Value::List(b)) => Arc::ptr_eq(a, b),
            (Value::FuncNative(a), Value::FuncNative(b)) => Arc::ptr_eq(a, b),
            (Value::Func(a), Value::Func(b)) => Arc::ptr_eq(a, b),
            (_, _) => false,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Nil
    }
}

#[derive(Debug, PartialEq)]
pub enum ZapErr {
    Msg(std::string::String),
}

pub fn error_msg(msg: &str) -> ZapErr {
    ZapErr::Msg(msg.to_string())
}

//
// ZapFn
//

#[derive(Debug)]
pub struct Closure {
    outers: Vec<Outer>,
    chunk: Arc<Chunk>,
}

#[derive(Debug)]
pub struct ZapFn {
    pub locals: Vec<Value>,
    pub chunk: Arc<Chunk>,
}

impl ZapFn {
    pub fn new(scope_size: usize, chunk: Chunk) -> Value {
        Value::Func(Arc::new(ZapFn {
            locals: vec![Value::Nil; scope_size],
            chunk: Arc::new(chunk),
        }))
    }

    pub fn new_closure(outers: Vec<Outer>, chunk: Chunk) -> Value {
        Value::Closure(Arc::new(Closure {
            outers,
            chunk: Arc::new(chunk),
        }))
    }

    pub fn from_closure(closure: Arc<Closure>, callframes: &[CallFrame], stack: &[Value]) -> Value {
        let mut locals = vec![Value::default(); closure.chunk.scope_size];

        for outer in &closure.outers {
            unsafe {
                let base = if outer.level == 0 {
                    0
                } else {
                    callframes.get_unchecked(outer.level - 1).get_ret()
                };
                let val = stack.get_unchecked(base + outer.position).clone();
                ptr::write(locals.as_mut_ptr().add(outer.dest.into()), val);
            }
        }

        Value::Func(Arc::new(ZapFn {
            locals,
            chunk: closure.chunk.clone(),
        }))
    }
}

pub struct ZapFnNative {
    pub name: String,
    pub func: fn(&[Value]) -> Result<Value>,
}

impl ZapFnNative {
    pub fn new(name: String, func: fn(&[Value]) -> Result<Value>) -> Arc<ZapFnNative> {
        Arc::new(ZapFnNative { name, func })
    }
}
