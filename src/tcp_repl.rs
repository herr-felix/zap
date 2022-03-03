mod env;
mod eval;
mod printer;
mod reader;
mod repl;
mod types;
mod core;

use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::repl::start_repl;

fn handle_client(mut stream: TcpStream) {
    let mut stream_out = stream.try_clone().unwrap();

    start_repl(&mut stream, &mut stream_out).unwrap();
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:2020")?;

    // accept connections and process them serially
    for stream in listener.incoming() {
        let stream = stream?;
        thread::spawn(move || {
            handle_client(stream);
        });
    }
    Ok(())
}
