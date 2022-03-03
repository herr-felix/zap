mod env;
mod eval;
mod printer;
mod reader;
mod repl;
mod types;

mod core;

use std::io;

use crate::repl::start_repl;

fn main() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    start_repl(&mut stdin, &mut stdout).unwrap();
}
