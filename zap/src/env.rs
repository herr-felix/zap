use fnv::FnvHashMap;
use smartstring::alias::String;
use std::mem;

use crate::types::{error, ZapExp, ZapFn, ZapFnRef, ZapResult};

type Scope = FnvHashMap<String, ZapExp>;

pub trait Env {
    fn push(&mut self);
    fn pop(&mut self);
    fn get(&self, symbol: &ZapExp) -> ZapResult;
    fn set(&mut self, key: String, val: ZapExp);
    fn reg_fn(&mut self, symbol: &str, f: ZapFnRef);
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
    fn get(&self, symbol: &ZapExp) -> ZapResult {
        if let ZapExp::Symbol(ref key) = symbol {
            self.scope
                .get(key)
                .cloned()
                .ok_or_else(|| error(format!("symbol '{}' not in scope.", key).as_str()))
        } else {
            Err(error("env.get: only symbols can be used as keys."))
        }
    }

    fn set(&mut self, key: String, val: ZapExp) {
        if let Some(prev) = self.scope.insert(key.clone(), val) {
            // We put the previous value in shadow, if there was any.
            self.shadow.entry(key).or_insert(prev);
        }
    }

    fn reg_fn(&mut self, symbol: &str, f: ZapFnRef) {
        self.scope.insert(
            String::from(symbol),
            ZapExp::Func(ZapFn::new(String::from(symbol), f)),
        );
    }
}
