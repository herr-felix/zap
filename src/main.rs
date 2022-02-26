use std::fmt;

mod types;
use crate::types::{ZapExp, ZapErr};
mod reader;
use crate::reader::{Reader, tokenize, read_form};
mod printer;


fn read_all(rdr: &mut Reader) -> Result<Vec<ZapExp>, ZapErr> {
    let mut forms = Vec::new();
    while rdr.peek() != None {
        forms.push(read_form(rdr)?);
    }
    return Ok(forms)
}

fn main() {
    let tokens = tokenize(r##"(concat "hello " "world" " \"escaped\" ") ; comment 1
                            ; comment 2
                          ~@(test~test)
                          (+ 2 1 2 3)as;
df"##.to_string());

    let exprs = read_all(&mut Reader{pos: 0, tokens}).unwrap();

    for ex in exprs {
        println!("{}", ex.pr_str());
    }
}
