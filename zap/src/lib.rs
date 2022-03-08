pub mod env;
pub mod eval;
pub mod printer;
pub mod reader;
pub mod types;

pub trait Evaluator {
    fn eval<E: env::Env>(&mut self, env: &mut E);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
