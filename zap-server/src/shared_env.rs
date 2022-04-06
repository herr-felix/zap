use std::sync::{Arc, RwLock};

use zap::env::{symbols, Env, Scope, SymbolTable};
use zap::{error_msg, Result, String, Symbol, Value};

// SharedEnv, a shared environement.
// Every changes to the env made from the runtime are
// made available to all other shared envs on the same
// hub.

pub struct SharedEnv {
    globals: Scope,
    shared_globals: Arc<RwLock<Scope>>,
    symbols: Arc<RwLock<SymbolTable>>,
}

impl Default for SharedEnv {
    fn default() -> Self {
        let mut this = SharedEnv {
            globals: Scope::default(),
            shared_globals: Arc::new(RwLock::new(Scope::default())),
            symbols: Arc::new(RwLock::new(SymbolTable::default())),
        };

        for s in symbols::DEFAULT_SYMBOLS {
            this.reg_symbol(String::from(s));
        }

        this
    }
}

impl Clone for SharedEnv {
    fn clone(&self) -> Self {
        SharedEnv {
            globals: self.shared_globals.read().unwrap().clone(), // I don't like copying all the globals every time we get a new env
            shared_globals: self.shared_globals.clone(),
            symbols: self.symbols.clone(),
        }
    }
}

impl Env for SharedEnv {
    #[inline(always)]
    fn get_by_id(&self, id: Symbol) -> Result<Value> {
        match unsafe { self.globals.get_unchecked(id as usize) } {
            Some(val) => Ok(val.clone()),
            None => Err(match self.get_symbol(id) {
                Ok(s) => error_msg(format!("symbol '{}' not in scope.", s).as_str()),
                Err(err) => err,
            }),
        }
    }

    fn set(&mut self, key: &Value, val: &Value) -> Result<()> {
        if let Value::Symbol(id) = key {
            self.shared_globals.write().unwrap()[*id as usize] = Some(val.clone());
            self.globals[*id as usize] = Some(val.clone());
            Ok(())
        } else {
            Err(error_msg("Env set: only symbols can be used as keys."))
        }
    }

    fn reg_symbol(&mut self, s: String) -> Value {
        let mut symbols = self.symbols.write().unwrap();
        let len = symbols.len();
        let id = symbols.entry(s).or_insert_with(|| {
            self.shared_globals.write().unwrap().push(None);
            self.globals.push(None);
            len.try_into().unwrap()
        });
        Value::Symbol(*id)
    }

    fn get_symbol(&self, id: Symbol) -> Result<String> {
        let symbols = self.symbols.read().unwrap();
        symbols
            .iter()
            .find(|(_, v)| **v == id)
            .map(|(k, _)| k.clone())
            .ok_or_else(|| error_msg(format!("No known symbol for id={}", id).as_str()))
    }
}
