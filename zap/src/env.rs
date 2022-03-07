use fnv::FnvHashMap;
use std::mem;

use crate::types::{error, ZapExp, ZapFn, ZapResult};

type Scope = FnvHashMap<String, ZapExp>;

pub trait Env {
    fn push(&mut self);
    fn pop(&mut self);
    fn get(&self, key: &str) -> ZapResult;
    fn set(&mut self, key: String, val: ZapExp) -> ZapExp;
    fn reg_fn(&mut self, symbol: &str, f: ZapFn);
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

    fn get(&self, key: &str) -> ZapResult {
        self.scope
            .get(key)
            .cloned()
            .ok_or_else(|| error(format!("symbol '{}' not in scope.", key).as_str()))
    }

    fn set(&mut self, key: String, val: ZapExp) -> ZapExp {
        if let Some(prev) = self.scope.insert(key.clone(), val.clone()) {
            // We put the previous value in shadow, if there was any.
            self.shadow.entry(key).or_insert(prev);
        }
        val
    }

    fn reg_fn(&mut self, symbol: &str, f: ZapFn) {
        self.scope
            .insert(symbol.to_string(), ZapExp::Func(symbol.to_string(), f));
    }
}
