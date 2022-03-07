use fnv::FnvHashMap;
use std::borrow::BorrowMut;

use crate::types::{error, ZapExp, ZapFn, ZapResult};

pub struct Env {
    root: FnvHashMap<String, ZapExp>,
}

impl Env {
    pub fn new() -> Env {
        Env {
            root: FnvHashMap::<String, ZapExp>::default(),
        }
    }

    // TODO: Push and Pop and scope

    pub fn get(&self, key: &String) -> Option<ZapExp> {
        self.root.get(key).and_then(|val| Some(val.clone()))
    }

    pub fn reg_fn(&mut self, symbol: &str, f: ZapFn) {
        self.root
            .insert(symbol.to_string(), ZapExp::Func(symbol.to_string(), f));
    }

    pub fn set(&mut self, key: String, val: ZapExp) {
        self.root.borrow_mut().insert(key, val);
    }
}
