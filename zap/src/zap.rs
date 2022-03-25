use std::sync::Arc;

pub use smartstring::alias::String;

use crate::env::Env;
use crate::vm::Chunk;

pub type Symbol = usize;

pub type ZapList = Arc<Vec<Value>>;
pub type Result<T> = std::result::Result<T, ZapErr>;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Symbol(Symbol),
    Str(String),
    List(ZapList),
    Func(ZapFn),
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

    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }
}

impl core::ops::Add for Value {
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

// ZapFn

pub struct ZapFnNative {
    pub name: String,
    pub func: fn(&[Value]) -> Result<Value>,
}

#[derive(Clone)]
pub enum ZapFn {
    Native(Arc<ZapFnNative>),
    Chunk(Arc<Chunk>),
}

impl ZapFn {
    pub fn native(name: String, func: fn(&[Value]) -> Result<Value>) -> Value {
        Value::Func(ZapFn::Native(Arc::new(ZapFnNative { name, func })))
    }

    pub fn from_chunk(chunk: Arc<Chunk>) -> Value {
        Value::Func(ZapFn::Chunk(chunk))
    }
}

impl PartialEq for ZapFn {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ZapFn::Native(a), ZapFn::Native(b)) => a.name == b.name,
            (ZapFn::Chunk(a), ZapFn::Chunk(b)) => Arc::ptr_eq(a, b),
            (_, _) => false,
        }
    }
}

impl std::fmt::Debug for ZapFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZapFn::Native(_) => {
                write!(f, "Native func")
            }
            ZapFn::Chunk(_) => {
                write!(f, "<Chunk>")
            }
        }
    }
}
