mod repl;
mod shared_env;

//#[cfg(not(target_env = "msvc"))]
//use tikv_jemallocator::Jemalloc;
//#[global_allocator]
//static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

use crate::repl::start_repl;
use std::fs::remove_file;
use tokio::net::UnixListener;

use crate::shared_env::SharedEnv;

//#[cfg(not(target_env = "msvc"))]
//#[global_allocator]
//static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::io::Result<()> {
    let socket_file = "./zap.sock";
    remove_file(socket_file).ok(); // Cleanup the file
    let listener = UnixListener::bind(socket_file).unwrap();

    println!("Server listening.");

    let env = SharedEnv::default();

    // accept connections and process them serially
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let env = env.clone();
        tokio::spawn(async move {
            let (mut input, mut output) = stream.into_split();
            start_repl(&mut input, &mut output, env).await.ok();
        });
    }
}
