use crate::types::ZapExp;
use crate::env::Env;
use smartstring::alias::String;

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
            ZapExp::Symbol(s) => env.get_symbol(&s).unwrap().to_string(),
            ZapExp::Str(s) => format!("\"{}\"", escape_str(s.clone())),
            ZapExp::List(l) => pr_seq(l, "(", ")", env),
            ZapExp::Func(f) => format!("{:?}", f),
        }
    }
}

fn pr_seq<E: Env>(seq: &[ZapExp], start: &str, end: &str, env: &mut E) -> std::string::String {
    let strs: Vec<std::string::String> = seq.iter().map(|x| x.pr_str(env)).collect();
    format!("{}{}{}", start, strs.join(" "), end)
}
