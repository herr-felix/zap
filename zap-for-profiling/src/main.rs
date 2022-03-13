use zap::env::SandboxEnv;
use zap::eval::Evaluator;
use zap::reader::Reader;

fn main() {
    let mut reader = Reader::new();
    let mut env = SandboxEnv::default();

    zap_core::load(&mut env);

    let mut session = Evaluator::new(env);

    let src = "(define rec (fn (x) (if (= x 1000000) \"boom\" (rec (+ x 1))))) (rec 0) (rec 0)";

    reader.tokenize(src);

    while let Ok(Some(form)) = reader.read_form(session.get_env()) {
        if let Ok(result) = session.eval(form) {
            println!("{}", result.pr_str(session.get_env()));
        }
    }
}
