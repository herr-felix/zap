use crate::types::ZapExp;
use smartstring::alias::String;

fn escape_str(s: String) -> std::string::String {
    s.replace('"', "\\\"")
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
}

impl ZapExp {
    pub fn pr_str(&self) -> std::string::String {
        match self {
            ZapExp::Nil => std::string::String::from("nil"),
            ZapExp::Bool(true) => std::string::String::from("true"),
            ZapExp::Bool(false) => std::string::String::from("false"),
            ZapExp::Number(f) => format!("{}", f),
            ZapExp::Symbol(s) => s.to_string(), // TODO: Find string for symbol in env
            ZapExp::Str(s) => format!("\"{}\"", escape_str(s.clone())),
            ZapExp::List(l) => pr_seq(l, "(", ")"),
            ZapExp::Func(f) => format!("{:?}", f),
        }
    }
}

fn pr_seq(seq: &[ZapExp], start: &str, end: &str) -> std::string::String {
    let strs: Vec<std::string::String> = seq.iter().map(|x| x.pr_str()).collect();
    format!("{}{}{}", start, strs.join(" "), end)
}
