use zap::env::Env;
use zap::{error_msg, Result, Value};

fn is_float(args: &[Value]) -> Result<Value> {
    if args.is_empty() {
        return Err(error_msg("'float?' requires at least 1 argument."));
    }
    for v in args {
        match v {
            Value::Number(_) => continue,
            _ => return Ok(Value::Bool(false)),
        }
    }
    Ok(Value::Bool(true))
}

fn is_false(args: &[Value]) -> Result<Value> {
    if args.is_empty() {
        return Err(error_msg("'false?' requires at least 1 argument."));
    }
    for v in args {
        match v {
            Value::Bool(false) => continue,
            _ => return Ok(Value::Bool(false)),
        }
    }
    Ok(Value::Bool(true))
}

pub fn load<E: Env>(env: &mut E) -> Result<()> {
    env.reg_fn("float?", is_float)?;
    env.reg_fn("false?", is_false)?;
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::load;
    use zap::env::SandboxEnv;
    use zap::tests::run_exp;

    fn test_exp_core(src: &str, expected: &str) {
        let mut env = SandboxEnv::default();
        load(&mut env);
        assert_eq!(run_exp(src, env).unwrap(), expected);
    }

    #[test]
    fn is_false() {
        test_exp_core("(false? false)", "true");
        test_exp_core("(false? nil)", "false");
        test_exp_core("(false? 12)", "false");
        test_exp_core("(false? true)", "false");
        test_exp_core("(false? ())", "false");
        test_exp_core("(false? (false? true))", "true");
    }

    #[test]
    fn is_float() {
        test_exp_core("(float? false)", "false");
        test_exp_core("(float? nil)", "false");
        test_exp_core("(float? \"test\")", "false");
        test_exp_core("(float? 12)", "true");
        test_exp_core("(float? true)", "false");
        test_exp_core("(float? ())", "false");
    }
}
