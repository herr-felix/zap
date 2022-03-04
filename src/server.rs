mod env;
mod eval;
mod printer;
mod reader;
mod repl;
mod types;
mod core;

use tokio::net::{TcpListener};

use crate::repl::start_repl;


#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:2020").await.unwrap();

    // accept connections and process them serially
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            start_repl(socket).await.unwrap();
        });
    }
}
