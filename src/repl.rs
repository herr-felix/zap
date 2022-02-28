use std::io;

use crate::reader::Reader;
use crate::types::ZapErr;

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

        loop {
            let n = input.read(&mut buf[..])?;

            let src = std::str::from_utf8(&buf[..n]).unwrap();
            reader.tokenize(src);

            loop {
                match reader.read_form() {
                    Ok(Some(form)) => {
                        output.write_fmt(format_args!("{}\n", form.pr_str()))?;
                    }
                    Ok(None) => break,
                    Err(ZapErr::Msg(err)) => {
                        output.write_fmt(format_args!("Error: {}\n", err))?;
                    }
                }
            }

            if src.ends_with('\n') {
                break;
            }
        }
    }
}
