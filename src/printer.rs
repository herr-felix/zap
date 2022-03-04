
use crate::types::ZapExp;

fn escape_str(s: String) -> String {
    s.replace("\"", "\\\"")
        .replace("\\", "\\\\")
        .replace("\n", "\\n")
}

impl ZapExp {
    pub fn pr_str(&self) -> String {
        match self {
            ZapExp::Nil => String::from("nil"),
            ZapExp::Bool(true) => String::from("true"),
            ZapExp::Bool(false) => String::from("false"),
            ZapExp::Number(f) => format!("{}", f),
            ZapExp::Symbol(s) => s.clone(),
            ZapExp::Str(s) => format!("\"{}\"", escape_str(s.clone())), // TODO: Escape string
            ZapExp::List(l) => pr_seq(l, "(", ")"),
            ZapExp::Func(f, _) => format!("<Func {}>", f),
        }
    }
}

fn pr_seq(seq: &Vec<ZapExp>, start: &str, end: &str) -> String {
    let strs: Vec<String> = seq.iter().map(|x| x.pr_str()).collect();
    format!("{}{}{}", start, strs.join(" "), end)
}
