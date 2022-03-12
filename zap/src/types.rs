use smartstring::alias::String;
use std::sync::Arc;

pub type ZapFnNative = fn(&[ZapExp]) -> ZapResult;

#[derive(Clone)]
pub enum ZapFn {
    Native(String, ZapFnNative),
    Func { args: ZapList, ast: ZapExp },
}

impl ZapFn {
    pub fn native(name: String, func: ZapFnNative) -> ZapExp {
        ZapExp::Func(Arc::new(ZapFn::Native(name, func)))
    }

    pub fn new_fn(args: ZapList, ast: ZapExp) -> ZapExp {
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
    Symbol(String),
    Number(f64),
    Str(String),
    List(ZapList),
    Func(Arc<ZapFn>),
}

impl ZapExp {
    pub fn new_list(list: Vec<ZapExp>) -> ZapList {
        Arc::new(list)
    }

    pub fn is_truish(&self) -> bool {
        !matches!(*self, ZapExp::Nil | ZapExp::Bool(false))
    }
}

impl Default for ZapExp {
    fn default() -> Self {
        ZapExp::Nil
    }
}

impl core::ops::Add for ZapExp {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if let (ZapExp::Number(a), ZapExp::Number(b)) = (self, other) {
            return ZapExp::Number(a + b);
        }
        ZapExp::Nil
    }
}

#[derive(Debug)]
pub enum ZapErr {
    Msg(std::string::String),
}

pub fn error(msg: &str) -> ZapErr {
    ZapErr::Msg(msg.to_string())
}

pub type ZapResult = Result<ZapExp, ZapErr>;
pub type ZapList = Arc<Vec<ZapExp>>;
