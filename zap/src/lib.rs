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
    use crate::env::SandboxEnv;
    use crate::eval::Evaluator;
    use crate::reader::Reader;
    use crate::types::String;

    fn run_exp(src: &str) -> String {
        let mut reader = Reader::new();
        reader.tokenize(src);

        reader.flush_token();

        let env = SandboxEnv::default();
        let mut session = Evaluator::new(env);

        let mut form = reader.read_form(session.get_env()).unwrap();
        let mut res = session.eval(form.unwrap()).unwrap();

        loop {
            form = reader.read_form(session.get_env()).unwrap();
            if form.is_none() {
                return String::from(res.pr_str(session.get_env()))
            }
            res = session.eval(form.unwrap()).unwrap();
        }

    }

    #[test]
    fn eval_number() {
        assert_eq!(run_exp("1"), "1");
    }

    #[test]
    fn eval_if() {
        assert_eq!(run_exp("(if 12 false true)"), "false");
    }

    #[test]
    fn eval_quoted_symbol() {
        assert_eq!(run_exp("'a"), "a");
    }

    #[test]
    fn eval_quoted_list() {
        assert_eq!(run_exp("'(1 2 3)"), "(1 2 3)");
    }

    #[test]
    fn eval_quasiquote_list() {
        assert_eq!(run_exp("(define x '(2 3)) `(1 x 4)"), "(1 x 4)");
    }

    #[test]
    fn eval_unquote_list() {
        assert_eq!(run_exp("(define D '(2 3)) `(1 ~D 4)"), "(1 (2 3) 4)");
    }

    #[test]
    fn eval_splice_unquote_list() {
        assert_eq!(run_exp(r#"(define c '(1 "b" "d"))"#), r#"(1 "b" "d")"#);
        assert_eq!(
            run_exp(r#"(define c '(1 "b" "d")) `(1 c 3)"#),
            r#"(1 c 3)"#
        );
        assert_eq!(
            run_exp(r#"(define c '(1 "b" "d")) `(1 ~@c 3)"#),
            r#"(1 1 "b" "d" 3)"#
        );
        assert_eq!(
            run_exp(r#"(define c '(1 "b" "d")) `(1 ~@c)"#),
            r#"(1 1 "b" "d")"#
        );
        assert_eq!(
            run_exp(r#"(define c '(1 "b" "d")) `(~@c 2)"#),
            r#"(1 "b" "d" 2)"#
        );
        assert_eq!(
            run_exp(r#"(define c '(1 "b" "d")) `(~@c ~@c)"#),
            r#"(1 "b" "d" 1 "b" "d")"#
        );
        assert_eq!(run_exp("(define x '(2 3)) `(1 ~@x 4)"), "(1 2 3 4)");
    }

    #[test]
    fn eval_let_quote_list() {
        assert_eq!(run_exp("(let (x '(2 3)) x)"), "(2 3)");
    }

    #[test]
    fn eval_let_unquote_list() {
        assert_eq!(run_exp("(let (x 0) `~x)"), "0");
    }

    #[test]
    fn eval_returned_function_with_lexical_scoping() {
        assert_eq!(run_exp("(define same-maker (fn (x) (fn () x))) (define same-4 (same-maker 4)) (same-4)"), "4");
    }
}
