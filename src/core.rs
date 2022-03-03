
use crate::types::{ZapExp, ZapResult, error};
use crate::types::ZapExp::{Number};
use crate::env::Env;


fn plus(args: &[ZapExp]) -> ZapResult {
    let mut sum = 0.0;
    for v in args {
        if let ZapExp::Number(x) = v {
            sum = sum+x;
        } else {
            return Err(error("+ can only add numbers."))
        }
    }
    Ok(Number(sum))
}


pub fn load(env: &mut Env) {
    env.set(ZapExp::Symbol("+".to_string()), ZapExp::Func("+".to_string(), plus)).unwrap();
}
