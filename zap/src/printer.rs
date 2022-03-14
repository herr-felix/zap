use crate::env::Env;
use crate::types::ZapExp;
use smartstring::alias::String;
use std::fmt;

fn escape_str(s: String) -> std::string::String {
    s.replace('"', "\\\"")
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
}

impl ZapExp {
    pub fn pr_str<E: Env>(&self, env: &mut E) -> std::string::String {
        match self {
            ZapExp::Nil => std::string::String::from("nil"),
            ZapExp::Bool(true) => std::string::String::from("true"),
            ZapExp::Bool(false) => std::string::String::from("false"),
            ZapExp::Number(f) => format!("{}", f),
            ZapExp::Symbol(s) => env.get_symbol(*s).unwrap().to_string(),
            ZapExp::Str(s) => format!("\"{}\"", escape_str(s.clone())),
            ZapExp::List(l) => pr_seq(l, "(", ")", env),
            ZapExp::Func(f) => format!("{:?}", f),
            ZapExp::DateTime(t) => format!("DateTime<{}>", t.to_rfc3339()),
            ZapExp::Duration(d) => format!("Duration<{}>", d),
        }
    }
}

fn pr_seq<E: Env>(seq: &[ZapExp], start: &str, end: &str, env: &mut E) -> std::string::String {
    let strs: Vec<std::string::String> = seq.iter().map(|x| x.pr_str(env)).collect();
    format!("{}{}{}", start, strs.join(" "), end)
}

impl std::fmt::Display for ZapExp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZapExp::Nil => write!(f, "nil"),
            ZapExp::Bool(true) => write!(f, "true"),
            ZapExp::Bool(false) => write!(f, "false"),
            ZapExp::Number(n) => write!(f, "{}", n),
            ZapExp::Symbol(s) => write!(f, "Symbol#{}", s),
            ZapExp::Str(s) => write!(f, "\"{}\"", escape_str(s.clone())),
            ZapExp::List(l) => write!(f, "{}", debug_seq(l, "(", ")")),
            ZapExp::Func(func) => write!(f, "{:?}", func),
            ZapExp::DateTime(t) => write!(f, "DateTime<{}>", t.to_rfc3339()),
            ZapExp::Duration(d) => write!(f, "Duration<{}>", d),
        }
    }
}

fn debug_seq(seq: &[ZapExp], start: &str, end: &str) -> std::string::String {
    let strs: Vec<std::string::String> = seq.iter().map(|x| format!("{:?}", x)).collect();
    format!("{}{}{}", start, strs.join(" "), end)
}
