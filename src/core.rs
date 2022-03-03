
use crate::types::{ZapExp, ZapResult, error};
use crate::types::ZapExp::{Number};
use crate::env::Env;


fn plus(args: &[ZapExp]) -> ZapResult {
    match args.iter().cloned().reduce(|acc, x| { acc + x }) {
        Some(Number(x)) => Ok(Number(x)),
        None => Ok(Number(0.0)),
        _ => Err(error("+ can only add numbers.")),
    }
}


pub fn load(env: &mut Env) {
    env.set(ZapExp::Symbol("+".to_string()), ZapExp::Func("+".to_string(), plus)).unwrap();
}
