pub mod env;
pub mod eval;
pub mod printer;
pub mod reader;
pub mod types;

pub trait Evaluator {
    fn eval<E: env::Env>(&mut self, env: &mut E);
}
