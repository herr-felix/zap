use crate::env::Env;
use crate::zap::Value;
use std::fmt;

fn escape_str(s: &str) -> String {
    s.replace('"', "\\\"")
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
}

impl Value {
    pub fn pr_str<E: Env>(&self, env: &mut E) -> String {
        match self {
            Value::Symbol(s) => env.get_symbol(*s).unwrap().to_string(),
            Value::List(l) => pr_seq(l, "(", ")", env),
            val => format!("{}", val),
        }
    }
}

fn pr_seq<E: Env>(seq: &[Value], start: &str, end: &str, env: &mut E) -> String {
    let strs: Vec<String> = seq.iter().map(|x| x.pr_str(env)).collect();
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
            Value::Str(s) => write!(f, "\"{}\"", escape_str(s)),
            Value::List(l) => write!(f, "{}", debug_seq(l, "(", ")")),
            Value::Func(_) => write!(f, "Func"),
            Value::FuncNative(func) => write!(f, "FuncNative<{}>", func.name),
        }
    }
}

fn debug_seq(seq: &[Value], start: &str, end: &str) -> String {
    let strs: Vec<String> = seq.iter().map(|x| format!("{}", x)).collect();
    format!("{}{}{}", start, strs.join(" "), end)
}
