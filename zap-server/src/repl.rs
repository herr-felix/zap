use std::time::Instant;

use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task;

use zap::env::SandboxEnv;
use zap::eval::Evaluator;
use zap::reader::Reader;
use zap::types::ZapErr;

pub async fn start_repl(stream: TcpStream) -> io::Result<()> {
    let (mut input, mut output) = stream.into_split();

    let mut buf = [0; 1024];

    let mut reader = Reader::new();
    let mut env = SandboxEnv::default();

    zap_core::load(&mut env);

    let mut session = Evaluator::new(env);

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
                    return Err(e);
                }
            };

            let src = std::str::from_utf8(&buf[..n]).unwrap();
            reader.tokenize(src);

            loop {
                match reader.read_form(session.get_env()) {
                    Ok(Some(form)) => {
                        let evaluator = &mut session;
                        let start = Instant::now();
                        let evaluated = task::block_in_place(move || evaluator.eval(form));
                        let end = Instant::now();

                        match evaluated {
                            Ok(result) => {
                                output
                                    .write(format!("{}\n", result.pr_str()).as_bytes())
                                    .await?;
                                output
                                    .write(format!("Evaluated in {:?}\n", end - start).as_bytes())
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
