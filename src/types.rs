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

impl ZapExp {
    pub fn is_truish(&self) -> bool {
        match *self {
            ZapExp::Nil => false,
            ZapExp::Bool(false) => false,
            _ => true,
        }
    }
}

impl core::ops::Add for ZapExp {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if let (ZapExp::Number(a), ZapExp::Number(b)) = (self, other) {
            return ZapExp::Number(a + b);
        }
        return ZapExp::Nil;
    }
}

#[derive(Debug)]
pub enum ZapErr {
    Msg(String),
}

pub fn error(msg: &str) -> ZapErr {
    ZapErr::Msg(msg.to_string())
}

pub type ZapResult = Result<ZapExp, ZapErr>;
