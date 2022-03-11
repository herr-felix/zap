use fnv::FnvHashMap;
use smartstring::alias::String;
use std::mem;

use crate::types::{error, ZapErr, ZapExp, ZapFn, ZapFnNative, ZapResult};

type Scope = FnvHashMap<String, ZapExp>;

pub trait Env {
    fn push(&mut self);
    fn pop(&mut self);
    fn get(&self, symbol: &String) -> ZapResult;
    fn set(&mut self, key: &ZapExp, val: ZapExp) -> Result<(), ZapErr>;
    fn reg_fn(&mut self, symbol: &str, f: ZapFnNative);
}

#[derive(Default)]
pub struct BasicEnv {
    scope: Scope,
    shadow: Scope,
    stack: Vec<Scope>,
}

impl Env for BasicEnv {
    fn push(&mut self) {
        self.stack.push(mem::take(&mut self.shadow));
    }

    fn pop(&mut self) {
        self.scope.extend(self.shadow.drain());

        if let Some(new_shadow) = self.stack.pop() {
            self.shadow = new_shadow;
        }
    }

    #[inline(always)]
    fn get(&self, key: &String) -> ZapResult {
        self.scope
            .get(key)
            .cloned()
            .ok_or_else(|| error(format!("symbol '{}' not in scope.", key).as_str()))
    }

    fn set(&mut self, key: &ZapExp, val: ZapExp) -> Result<(), ZapErr> {
        if let ZapExp::Symbol(s) = key {
            if let Some(prev) = self.scope.insert(s.clone(), val) {
                // We put the previous value in shadow, if there was any.
                self.shadow.entry(s.clone()).or_insert(prev);
            }
            Ok(())
        }
        else {
            Err(error("Env set: only symbols can be used as keys."))
        }
    }

    fn reg_fn(&mut self, symbol: &str, f: ZapFnNative) {
        self.scope.insert(
            String::from(symbol),
            ZapExp::Func(ZapFn::native(String::from(symbol), f)),
        );
    }
}
