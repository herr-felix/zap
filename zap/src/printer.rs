use crate::env::Env;
use crate::zap::{String, Value};
use std::fmt;

fn escape_str(s: String) -> std::string::String {
    s.replace('"', "\\\"")
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
}

impl Value {
    pub fn pr_str<E: Env>(&self, env: &mut E) -> std::string::String {
        match self {
            Value::Symbol(s) => env.get_symbol(*s).unwrap().to_string(),
            Value::List(l) => pr_seq(l, "(", ")", env),
            Value::Func(f) => format!("{:?}", f),
            val => format!("{}", val),
        }
    }
}

fn pr_seq<E: Env>(seq: &[Value], start: &str, end: &str, env: &mut E) -> std::string::String {
    let strs: Vec<std::string::String> = seq.iter().map(|x| x.pr_str(env)).collect();
    format!("{}{}{}", start, strs.join(" "), end)
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(true) => write!(f, "true"),
            Value::Bool(false) => write!(f, "false"),
            Value::Number(n) => write!(f, "{}", n),
            Value::Symbol(n) => write!(f, "Symbol#{}", n),
            Value::Str(s) => write!(f, "\"{}\"", escape_str(s.clone())),
            Value::List(l) => write!(f, "{}", debug_seq(l, "(", ")")),
            Value::Func(_) => write!(f, "Func"),
        }
    }
}

fn debug_seq(seq: &[Value], start: &str, end: &str) -> std::string::String {
    let strs: Vec<std::string::String> = seq.iter().map(|x| format!("{}", x)).collect();
    format!("{}{}{}", start, strs.join(" "), end)
}
