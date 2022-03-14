pub use chrono::{DateTime, Duration, Utc};
pub use smartstring::alias::String;
use std::sync::Arc;

pub type Symbol = usize;

pub type ZapResult = Result<ZapExp, ZapErr>;
pub type ZapList = Arc<Vec<ZapExp>>;

pub type ZapFnNative = fn(&[ZapExp]) -> ZapResult;

#[derive(Clone)]
pub enum ZapFn {
    Native(String, ZapFnNative),
    Func { args: Vec<Symbol>, ast: ZapExp },
}

impl ZapFn {
    pub fn native(name: String, func: ZapFnNative) -> ZapExp {
        ZapExp::Func(Arc::new(ZapFn::Native(name, func)))
    }

    pub fn new_fn(args: Vec<Symbol>, ast: ZapExp) -> ZapExp {
        ZapExp::Func(Arc::new(ZapFn::Func { args, ast }))
    }
}

impl PartialEq for ZapFn {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ZapFn::Native(a, _), ZapFn::Native(b, _)) => a == b,
            (
                ZapFn::Func {
                    args: args_a,
                    ast: ast_a,
                },
                ZapFn::Func {
                    args: args_b,
                    ast: ast_b,
                },
            ) => args_a == args_b && ast_a == ast_b,
            (_, _) => false,
        }
    }
}

impl std::fmt::Debug for ZapFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ZapFn::Native(name, _) => {
                write!(f, "Native func<{}>", name)
            }
            ZapFn::Func { args, ast: _ } => {
                write!(f, "Func <{}>", args.len())
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum ZapExp {
    Nil,
    Bool(bool),
    Symbol(Symbol),
    Number(f64),
    Str(String),
    List(ZapList),
    Func(Arc<ZapFn>),
    DateTime(DateTime<Utc>),
    Duration(Duration),
}

impl ZapExp {
    pub fn new_list(list: Vec<ZapExp>) -> ZapList {
        Arc::new(list)
    }

    pub fn is_truish(&self) -> bool {
        !matches!(self, ZapExp::Nil | ZapExp::Bool(false))
    }
}

impl Default for ZapExp {
    fn default() -> Self {
        ZapExp::Nil
    }
}

impl core::ops::Add for &ZapExp {
    type Output = ZapResult;

    #[inline(always)]
    fn add(self, other: Self) -> Self::Output {
        match (self, other) {
            (ZapExp::Number(a), ZapExp::Number(b)) => Ok(ZapExp::Number(a + b)),
            (a, b) => Err(error(format!("Can't add {} + {}", a, b).as_str())),
        }
    }
}

impl core::ops::Sub for &ZapExp {
    type Output = ZapResult;

    #[inline(always)]
    fn sub(self, other: Self) -> Self::Output {
        match (self, other) {
            (ZapExp::Number(a), ZapExp::Number(b)) => Ok(ZapExp::Number(a - b)),
            (ZapExp::DateTime(a), ZapExp::DateTime(b)) => Ok(ZapExp::Duration(*a - *b)),
            (a, b) => Err(error(format!("Can't substract {} - {}", a, b).as_str())),
        }
    }
}

impl core::ops::Mul for &ZapExp {
    type Output = ZapResult;

    #[inline(always)]
    fn mul(self, other: Self) -> Self::Output {
        match (self, other) {
            (ZapExp::Number(a), ZapExp::Number(b)) => Ok(ZapExp::Number(a * b)),
            (a, b) => Err(error(format!("Can't multiply {} - {}", a, b).as_str())),
        }
    }
}

impl From<bool> for ZapExp {
    fn from(b: bool) -> Self {
        ZapExp::Bool(b)
    }
}

#[derive(Debug)]
pub enum ZapErr {
    Msg(std::string::String),
}

pub fn error(msg: &str) -> ZapErr {
    ZapErr::Msg(msg.to_string())
}
