use std::time::Instant;

use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task;

use zap::compiler::compile;
use zap::env::SandboxEnv;
use zap::reader::Reader;
use zap::vm::VM;
use zap::ZapErr;

pub async fn start_repl(stream: TcpStream) -> io::Result<()> {
    let (mut input, mut output) = stream.into_split();

    let mut buf = [0; 1024];

    let mut reader = Reader::new();
    let mut env = SandboxEnv::default();

    zap_core::load(&mut env);

    let mut vm = VM::init();

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
                match reader.read_ast(&mut env) {
                    Ok(Some(form)) => {
                        let vm = &mut vm;
                        let env2 = &mut env;

                        let evaluated = task::block_in_place(move || {
                            let chunk = compile(form, env2)?;
                            let start = Instant::now();
                            let res = vm.run(chunk, env2)?;
                            let end = Instant::now();
                            println!("Evaluated in {:?}\n", end - start);
                            Ok(res)
                        });

                        match evaluated {
                            Ok(result) => {
                                let env = &mut env;
                                output
                                    .write(format!("{}\n", result.pr_str(env)).as_bytes())
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
