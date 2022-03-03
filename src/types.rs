use std::collections::VecDeque;

#[derive(Clone)]
pub enum ZapExp {
    Nil,
    Bool(bool),
    Symbol(String),
    Number(f64),
    Str(String),
    List(VecDeque<ZapExp>),
    Func(String, fn(&[ZapExp]) -> Result<ZapExp, ZapErr>),
}

#[derive(Debug)]
pub enum ZapErr {
    Msg(String),
}

pub type ZapResult = Result<ZapExp, ZapErr>;

pub fn error(msg: &str) -> ZapErr {
    ZapErr::Msg(msg.to_string())
}


impl core::ops::Add for ZapExp {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if let (ZapExp::Number(a), ZapExp::Number(b)) = (self, other) {
            return ZapExp::Number(a+b)
        }
        return ZapExp::Nil
    }
}
