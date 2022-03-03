use std::borrow::BorrowMut;
use std::collections::HashMap;

use crate::types::{error, ZapErr, ZapExp};

pub struct Env {
    root: HashMap<String, ZapExp>,
}

impl Env {
    pub fn new() -> Env {
        Env {
            root: HashMap::<String, ZapExp>::new(),
        }
    }

    pub fn get(&self, key: &String) -> Option<ZapExp> {
        self.root.get(key).and_then(|val| Some(val.clone()))
    }

    pub fn set(&mut self, key: ZapExp, val: ZapExp) -> Result<ZapExp, ZapErr> {
        match key {
            ZapExp::Symbol(s) => {
                self.root.borrow_mut().insert(s, val.clone());
                Ok(val)
            }
            _ => Err(error("Only symbols can be used for keys in env")),
        }
    }
}
