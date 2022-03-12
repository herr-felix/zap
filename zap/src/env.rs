use fnv::{FnvHashMap, FnvHashSet};
use smartstring::alias::String;

use crate::types::{error, ZapErr, ZapExp, ZapFn, ZapFnNative, ZapResult};

type Scope = FnvHashMap<String, ZapExp>;

pub trait Env {
    fn push(&mut self);
    fn pop(&mut self);
    fn get(&self, symbol: &String) -> ZapResult;
    fn set(&mut self, key: &ZapExp, val: &ZapExp) -> Result<(), ZapErr>;
    fn set_global(&mut self, key: &ZapExp, val: &ZapExp) -> Result<(), ZapErr>;
    fn reg_fn(&mut self, symbol: &str, f: ZapFnNative);
}

#[derive(Default)]
pub struct SandboxEnv {
    scope: Scope,
    locals: FnvHashSet<String>,
    stack: Vec<Scope>,
}

impl Env for SandboxEnv {
    #[inline(always)]
    fn push(&mut self) {
        let mut shadow = Scope::with_capacity_and_hasher(self.locals.len(), Default::default());

        for k in self.locals.drain() {
            let v = self.scope.get(&k).unwrap().clone();
            shadow.insert(k, v);
        }

        self.stack.push(shadow);
    }

    #[inline(always)]
    fn pop(&mut self) {
        if let Some(mut shadow) = self.stack.pop() {
            for k in self.locals.clone().drain() {
                if let Some(v) = shadow.remove(&k) {
                    self.scope.insert(k, v);
                } else {
                    self.locals.remove(&k);
                    self.scope.remove(&k);
                }
            }

            for (k, v) in shadow.drain() {
                self.locals.insert(k.clone());
                self.scope.insert(k, v);
            }
        } else {
            for k in self.locals.drain() {
                self.scope.remove(&k);
            }
        }
    }

    #[inline(always)]
    fn get(&self, key: &String) -> ZapResult {
        self.scope
            .get(key)
            .cloned()
            .ok_or_else(|| error(format!("symbol '{}' not in scope.", key).as_str()))
    }

    #[inline(always)]
    fn set(&mut self, key: &ZapExp, val: &ZapExp) -> Result<(), ZapErr> {
        if let ZapExp::Symbol(s) = key {
            if !self.locals.contains(s) {
                self.locals.insert(s.clone());
            }

            self.scope.insert(s.clone(), val.clone());

            Ok(())
        } else {
            Err(error("Env set: only symbols can be used as keys."))
        }
    }

    fn set_global(&mut self, key: &ZapExp, val: &ZapExp) -> Result<(), ZapErr> {
        if let ZapExp::Symbol(s) = key {
            self.scope.insert(s.clone(), val.clone());
            Ok(())
        } else {
            Err(error("Env set: only symbols can be used as keys."))
        }
    }

    fn reg_fn(&mut self, symbol: &str, f: ZapFnNative) {
        self.scope
            .insert(String::from(symbol), ZapFn::native(String::from(symbol), f));
    }
}
