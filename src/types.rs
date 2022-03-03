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

pub fn error(msg: &str) -> ZapErr {
    ZapErr::Msg(msg.to_string())
}
