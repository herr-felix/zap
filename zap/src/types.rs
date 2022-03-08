use smartstring::alias::String;

pub type ZapFnRef = fn(&[ZapExp]) -> ZapResult;

#[derive(Clone)]
pub struct ZapFn {
    name: String,
    func: ZapFnRef,
}

impl ZapFn {
    pub fn new(name: String, func: ZapFnRef) -> Box<Self> {
        Box::new(ZapFn { name, func })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Clone)]
pub enum ZapExp {
    Nil,
    Bool(bool),
    Symbol(String),
    Number(f64),
    Str(String),
    List(Vec<ZapExp>),
    Func(Box<ZapFn>),
}

impl ZapExp {
    pub fn is_truish(&self) -> bool {
        !matches!(*self, ZapExp::Nil | ZapExp::Bool(false))
    }

    #[inline(always)]
    pub async fn apply(list: Vec<ZapExp>) -> ZapResult {
        if let Some((first, args)) = list.split_first() {
            return match first {
                ZapExp::Func(f) => ((*f).func)(args),
                _ => Err(error("Only functions can be called.")),
            };
        }
        Err(error("Cannot evaluate a empty list."))
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
