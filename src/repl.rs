use std::io;

use crate::env::Env;
use crate::eval::eval;
use crate::reader::Reader;
use crate::types::{ZapErr, ZapExp};
use crate::core::load;

pub fn start_repl<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: io::Read,
    O: io::Write,
{
    let mut buf = [0; 4];
    let mut reader = Reader::new();

    loop {
        output.write("> ".as_bytes())?;
        output.flush()?;

        let mut env = Env::new();
        env.set(
            ZapExp::Symbol("f".to_string()),
            ZapExp::Str("Felix".to_string()),
        )
        .unwrap();

        load(&mut env);

        loop {
            let n = input.read(&mut buf[..])?;

            let src = std::str::from_utf8(&buf[..n]).unwrap();
            reader.tokenize(src);

            loop {
                match reader.read_form() {
                    Ok(Some(form)) => match eval(form, &mut env) {
                        Ok(result) => {
                            output.write_fmt(format_args!("{}\n", result.pr_str()))?;
                        }
                        Err(ZapErr::Msg(err)) => {
                            output.write_fmt(format_args!("Eval error: {}\n", err))?;
                        }
                    },
                    Ok(None) => break,
                    Err(ZapErr::Msg(err)) => {
                        output.write_fmt(format_args!("Reader error: {}\n", err))?;
                    }
                }
            }

            if src.ends_with('\n') {
                break;
            }
        }
    }
}
