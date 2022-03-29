#[warn(clippy::pedantic)]
#[allow(clippy::missing_errors_doc)]
pub mod compiler;
pub mod env;
pub mod printer;
pub mod reader;
pub mod vm;
pub mod zap;

pub use crate::zap::*;

#[cfg(debug_assertions)]
pub mod tests {
    use crate::compiler::compile;
    use crate::env::SandboxEnv;
    use crate::reader::Reader;
    use crate::vm::{Op, VM};
    use crate::zap::{Result, String, Value, ZapErr};

    pub fn run_exp(src: &str, mut env: SandboxEnv) -> Result<String> {
        let mut reader = Reader::new();

        dbg!(src);
        reader.tokenize(src);
        reader.flush_token();

        let mut vm = VM::init();

        let mut ast = reader.read_ast(&mut env)?;
        let mut chunk = compile(ast.unwrap())?;
        let mut res = vm.run(chunk, &mut env)?;

        loop {
            ast = reader.read_ast(&mut env)?;
            if ast.is_none() {
                return Ok(String::from(res.to_string(&mut env)));
            }
            chunk = compile(ast.unwrap())?;
            res = vm.run(chunk, &mut env)?;
        }
    }

    pub fn test_exp(src: &str, expected: &str) {
        let env = SandboxEnv::default();
        assert_eq!(run_exp(src, env).unwrap(), expected);
    }

    #[test]
    fn op_size() {
        assert_eq!(std::mem::size_of::<Op>(), 4)
    }

    #[test]
    fn value_size() {
        assert_eq!(std::mem::size_of::<Value>(), 32)
    }

    #[test]
    fn eval_number() {
        test_exp("1", "1");
    }

    #[test]
    fn eval_string() {
        test_exp("\"test\"", "\"test\"");
    }

    #[test]
    fn eval_bool() {
        test_exp("false", "false");
        test_exp("true", "true");
    }

    #[test]
    fn eval_empty_list() {
        test_exp("()", "()");
    }

    #[test]
    fn eval_if() {
        test_exp("(if true 10 20)", "10");
        test_exp("(if false 10 20)", "20");
        test_exp("(if nil false true)", "true");
    }

    #[test]
    fn eval_do() {
        test_exp("(do 1 2 3)", "3");
    }

    #[test]
    fn lookup_symbol() {
        let env = SandboxEnv::default();
        assert_eq!(
            run_exp("gg", env),
            Err(ZapErr::Msg("symbol 'gg' not in scope.".to_string()))
        );
    }

    #[test]
    fn eval_def() {
        test_exp("(def x 3)", "3");
    }

    #[test]
    fn eval_fn() {
        test_exp("((fn (x) x) 4)", "4");
    }

    #[test]
    fn add_numbers() {
        test_exp("(+)", "0");
        test_exp("(+ 8)", "8");
        test_exp("(+ 1 2)", "3");
        test_exp("(+ 1 2 2)", "5");
        test_exp("(+ 1 2 3 (+ 4 2))", "12");
    }

    #[test]
    fn eval_eq() {
        test_exp("(= 1 2)", "false");
        test_exp("(= nil false)", "false");
        test_exp("(= false false)", "true");
    }
}
