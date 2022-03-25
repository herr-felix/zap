pub mod compiler;
pub mod env;
pub mod printer;
pub mod reader;
pub mod vm;
pub mod zap;

pub use crate::zap::*;

#[cfg(test)]
mod tests {
    use crate::compiler::compile;
    use crate::env::SandboxEnv;
    use crate::reader::Reader;
    use crate::vm::{Op, VM};
    use crate::zap::{Result, String, Value, ZapErr};

    fn run_exp(src: &str) -> Result<String> {
        let mut reader = Reader::new();

        dbg!(src);
        reader.tokenize(src);
        reader.flush_token();

        let mut env = SandboxEnv::default();
        let mut vm = VM::init();

        let mut ast = reader.read_ast(&mut env)?;
        let mut chunk = compile(ast.unwrap(), &mut env)?;
        let mut res = vm.run(chunk, &mut env)?;

        loop {
            ast = reader.read_ast(&mut env)?;
            if ast.is_none() {
                return Ok(String::from(res.to_string(&mut env)));
            }
            chunk = compile(ast.unwrap(), &mut env)?;
            res = vm.run(chunk, &mut env)?;
        }
    }

    fn test_exp(src: &str, expected: &str) {
        assert_eq!(run_exp(src).unwrap(), expected);
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
    fn add_numbers() {
        test_exp("(+)", "0");
        test_exp("(+ 8)", "8");
        test_exp("(+ 1 2)", "3");
        test_exp("(+ 1 2 2)", "5");
        test_exp("(+ 1 2 3 (+ 4 2))", "12");
    }

    #[test]
    fn eval_if() {
        test_exp("(if true 10 20)", "10");
        test_exp("(if false 10 20)", "20");
        test_exp("(if nil false true)", "true");
        test_exp("(if (+ 1 2) false true)", "false");
        test_exp("(if (+ 1 2) (+ 1 2) true)", "3");
        test_exp("(if false (+ 1 2) (+ 2 2))", "4");
    }

    #[test]
    fn eval_nested() {
        test_exp("(+ 1 2 3 (if false 5 (+ 4 2)))", "12");
    }

    #[test]
    fn eval_do() {
        test_exp("(do 1 2 3)", "3");
        test_exp("(do 1 (+ 1 2 3) (if false 2 4))", "4");
    }

    #[test]
    fn lookup_symbol() {
        assert_eq!(
            run_exp("gg"),
            Err(ZapErr::Msg("symbol 'gg' not in scope.".to_string()))
        );
    }

    #[test]
    fn eval_def() {
        test_exp("(def x 3)", "3");
        test_exp("(def x 3) (+ x 2)", "5");
        test_exp("(def x 3) (def y 5) (+ x y)", "8");
        test_exp("(def x (+ 1 3)) (def y 5) (+ x y)", "9");
    }
}
