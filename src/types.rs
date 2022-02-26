
#[derive(Debug)]
pub enum ZapErr {
    Msg(String),
}

#[derive(Clone)]
pub enum ZapExp {
    Nil,
    Bool(bool),
    Symbol(String),
    Number(f64),
    Str(String),
    List(Vec<ZapExp>),
    Func(String, fn(&[ZapExp]) -> Result<ZapExp, ZapErr>),
}
