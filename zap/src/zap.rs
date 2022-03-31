use std::sync::Arc;

pub use smartstring::alias::String;

use crate::env::Env;
use crate::vm::Chunk;

pub type Symbol = u32;

pub type ZapList = Arc<Vec<Value>>;
pub type Result<T> = std::result::Result<T, ZapErr>;

#[repr(align(32))]
#[derive(Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    Symbol(Symbol),
    Str(String),
    List(ZapList),
    FuncNative(Arc<ZapFnNative>),
    Func(Arc<Chunk>),
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

#[derive(Clone)]
pub struct ZapFnNative {
    pub name: String,
    pub func: fn(&[Value]) -> Result<Value>,
}

impl ZapFnNative {
    pub fn new(name: String, func: fn(&[Value]) -> Result<Value>) -> Arc<ZapFnNative> {
        Arc::new(ZapFnNative { name, func })
    }
}

impl PartialEq for ZapFnNative {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
