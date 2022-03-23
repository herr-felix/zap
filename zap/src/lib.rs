pub mod compiler;
pub mod env;
pub mod printer;
pub mod reader;
pub mod vm;
pub mod zap;

pub trait Evaluator {
    fn eval<E: env::Env>(&mut self, env: &mut E);
}

#[cfg(test)]
mod tests {
    use crate::compiler::compile;
    use crate::env::SandboxEnv;
    use crate::reader::Reader;
    use crate::vm::VM;
    use crate::zap::String;

    fn run_exp(src: &str) -> String {
        let mut reader = Reader::new();

        reader.tokenize(src);
        reader.flush_token();

        let mut env = SandboxEnv::default();
        let mut vm = VM::init();

        let mut ast = reader.read_ast(&mut env).unwrap();
        let mut chunk = compile(ast.unwrap(), &mut env).unwrap();
        let mut res = vm.run(chunk, &mut env).unwrap();

        loop {
            ast = reader.read_ast(&mut env).unwrap();
            if ast.is_none() {
                return String::from(res.to_string(&mut env));
            }
            chunk = compile(ast.unwrap(), &mut env).unwrap();
            res = vm.run(chunk, &mut env).unwrap();
        }
    }

    #[test]
    fn eval_number() {
        assert_eq!(run_exp("1"), "1");
    }

    #[test]
    fn eval_empty_list() {
        assert_eq!(run_exp("()"), "()");
    }

    #[test]
    fn add_numbers() {
        assert_eq!(run_exp("(+)"), "0");
        assert_eq!(run_exp("(+ 8)"), "8");
        assert_eq!(run_exp("(+ 1 2)"), "3");
        assert_eq!(run_exp("(+ 1 2 2)"), "5");
        assert_eq!(run_exp("(+ 1 2 3 (+ 4 2))"), "12");
    }
}
