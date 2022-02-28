mod types;
mod reader;
mod printer;

use std::io;
use std::io::Write;

use crate::reader::Reader;
use crate::types::ZapErr;

fn repl() -> io::Result<()> {

    let mut reader = Reader::new();
    let mut line = String::new();
    let stdin = io::stdin();


    loop {
        print!("> ");
        io::stdout().flush();

        stdin.read_line(&mut line)?;

        reader.tokenize(line.as_str());

        match reader.read_form() {
            Ok(Some(form)) => {           
                println!("{}", form.pr_str());
            },
            Ok(None) => {},
            Err(ZapErr::Msg(err)) => {
                println!("Error: {}", err);
            },
        }
        line.truncate(0);
        
    }


}

fn main() {
    let _src = r##"(concat "hello " "world" " \"escaped\" " (and true false "asdfdsad")) ; comment 1
                            ; comment 2
                          ~@(test~test)
                          (+ 2 1 2 3)as;
df"##;

    let mut reader = Reader::new();

    
    while let Some(ex) = reader.read_form().unwrap() {
        println!("{}", ex.pr_str());
    }

    repl().unwrap();


}
