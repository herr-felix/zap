//#![feature(test)]

//use zap::env::SandboxEnv;
//use zap::eval::Evaluator;
//use zap::reader::Reader;

fn main() {
    //    let mut reader = Reader::new();
    //    let mut env = SandboxEnv::default();
    //
    //    zap_core::load(&mut env);
    //
    //    let mut session = Evaluator::new(env);
    //
    //    let src = "(define rec (fn (x) (if (= x 1000000) \"boom\" (rec (+ x 1))))) (rec 0) (rec 0)";
    //
    //    reader.tokenize(src);
    //
    //    while let Ok(Some(form)) = reader.read_form(session.get_env()) {
    //        if let Ok(result) = session.eval(form) {
    //            println!("{}", result.pr_str(session.get_env()));
    //        }
    //    }
}
//
//extern crate test;
//
//#[cfg(test)]
//mod tests {
//
//    use test::Bencher;
//    use zap::env::SandboxEnv;
//    use zap::eval::Evaluator;
//    use zap::reader::Reader;
//
//    #[bench]
//    fn bench_plus_1_2_3(b: &mut Bencher) {
//        let mut reader = Reader::new();
//        let mut env = SandboxEnv::default();
//
//        zap_core::load(&mut env);
//
//        let mut session = Evaluator::new(env);
//
//        let src = "(define rec (fn (x) (if (= x 1000) nil (rec (+ x 1)))))(+ 1 2 rec 03)";
//
//        reader.tokenize(src);
//        reader.read_form(session.get_env());
//
//        if let Ok(Some(form)) = reader.read_form(session.get_env()) {
//            b.iter(|| session.eval(form.clone()))
//        }
//    }
//}
