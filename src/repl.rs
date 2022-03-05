use std::time::Instant;

use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::core::load;
use crate::env::Env;
use crate::eval::{self, eval_exp};
use crate::reader::Reader;
use crate::types::{ZapErr, ZapExp};

pub async fn start_repl(stream: TcpStream) -> io::Result<()> {
    let (mut input, mut output) = stream.into_split();
    let mut buf = [0; 1024];
    let mut reader = Reader::new();

    let mut env = Env::new();
    env.set(
        ZapExp::Symbol("f".to_string()),
        ZapExp::Str("Felix".to_string()),
    )
    .unwrap();

    let mut stack = eval::new_stack(32);

    load(&mut env);

    loop {
        output.write("> ".as_bytes()).await?;
        output.flush().await?;

        loop {
            let n = match input.read(&mut buf[..]).await {
                Ok(0) => return Ok(()),
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            };

            let src = std::str::from_utf8(&buf[..n]).unwrap();
            reader.tokenize(src);

            loop {
                match reader.read_form() {
                    Ok(Some(form)) => {
                        output
                            .write(format!("Reader: {}\n", form.pr_str()).as_bytes())
                            .await?;
                        let start = Instant::now();
                        match eval_exp(&mut stack, form, &mut env) {
                            Ok(result) => {
                                let end = Instant::now();
                                output
                                    .write(format!("Evaluated in {:?}\n", end - start).as_bytes())
                                    .await?;
                                output
                                    .write(format!("{}\n", result.pr_str()).as_bytes())
                                    .await?;
                            }
                            Err(ZapErr::Msg(err)) => {
                                output
                                    .write(format!("Eval error: {}\n", err).as_bytes())
                                    .await?;
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(ZapErr::Msg(err)) => {
                        output
                            .write(format!("Reader error: {}\n", err).as_bytes())
                            .await?;
                    }
                }
            }

            if src.ends_with('\n') {
                break;
            }
        }
    }
}
