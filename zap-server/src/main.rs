mod repl;

use tokio::net::TcpListener;

use crate::repl::start_repl;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:2020").await.unwrap();

    println!("Server listening.");

    // accept connections and process them serially
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            start_repl(socket).await.unwrap();
        });
    }
}
