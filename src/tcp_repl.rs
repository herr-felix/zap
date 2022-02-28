mod printer;
mod reader;
mod types;

use std::io;

use crate::reader::Reader;
use crate::types::ZapErr;
use std::net::{TcpListener, TcpStream};

fn repl<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: io::Read,
    O: io::Write,
{
    let mut buf = [0; 128];
    let mut reader = Reader::new();

    loop {
        output.write("> ".as_bytes())?;
        output.flush()?;

        let n = input.read(&mut buf[..])?;

        let src = std::str::from_utf8(&buf[..n]).unwrap();

        reader.tokenize(src);

        match reader.read_form() {
            Ok(Some(form)) => {
                output.write_fmt(format_args!("{}\n", form.pr_str()));
            }
            Ok(None) => {}
            Err(ZapErr::Msg(err)) => {
                output.write_fmt(format_args!("Error: {}\n", err));
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut stream_out = stream.try_clone().unwrap();

    repl(&mut stream, &mut stream_out).unwrap();
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:2020")?;

    // accept connections and process them serially
    for stream in listener.incoming() {
        handle_client(stream?);
    }
    Ok(())
}
