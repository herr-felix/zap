mod printer;
mod reader;
mod repl;
mod types;

use std::io;

use crate::repl::start_repl;

fn main() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    start_repl(&mut stdin, &mut stdout).unwrap();
}
