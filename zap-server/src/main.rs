mod repl;

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

use tokio::net::TcpListener;

use crate::repl::start_repl;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main(flavor = "multi_thread")]
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
