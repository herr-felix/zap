
extern crate test;
use test::Bencher;

use crate::env::Env;
use crate::eval::eval;
use crate::types::ZapExp;


#[bench]
fn bench_eval_number(b: &mut Bencher) {
    let mut env = Env::new();
    let exp = ZapExp::Number(32.3);

    b.iter(|| {
        eval(exp, &mut env);
    });
}
