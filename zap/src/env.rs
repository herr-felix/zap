use fxhash::{FxHashMap, FxHashSet};
use smartstring::alias::String;

use crate::types::{error, Symbol, ZapErr, ZapExp, ZapFn, ZapFnNative, ZapResult};

type Scope = FxHashMap<Symbol, ZapExp>;
type SymbolTable = FxHashMap<String, Symbol>;

// TODO: Make sures all the default symbols (for special forms) are here.
// TODO: Make a macro that generate const Symbol for each default symbols.
const DEFAULT_SYMBOLS: [&str; 6] = ["if", "let", "fn", "do", "define", "quote"];
pub mod symbols {
    use crate::types::Symbol;

    pub const IF: Symbol = 0;
    pub const LET: Symbol = 1;
    pub const FN: Symbol = 2;
    pub const DO: Symbol = 3;
    pub const DEFINE: Symbol = 4;
    pub const QUOTE: Symbol = 5;
}

pub trait Env {
    fn push(&mut self);
    fn pop(&mut self);
    fn get(&self, symbol: &Symbol) -> ZapResult;
    fn set(&mut self, key: &ZapExp, val: &ZapExp) -> Result<(), ZapErr>;
    fn set_global(&mut self, key: &ZapExp, val: &ZapExp) -> Result<(), ZapErr>;
    fn reg_symbol(&mut self, s: String) -> ZapExp;
    fn get_symbol(&self, key: &Symbol) -> Result<String, ZapErr>;
    fn reg_fn(&mut self, symbol: &str, f: ZapFnNative);
}

pub struct SandboxEnv {
    scope: Scope,
    locals: FxHashSet<Symbol>,
    stack: Vec<Scope>,
    symbols: SymbolTable,
}

impl Default for SandboxEnv {
    fn default() -> Self {
        let mut this = SandboxEnv {
            scope: Scope::default(),
            locals: FxHashSet::<Symbol>::default(),
            stack: Vec::<Scope>::default(),
            symbols: SymbolTable::default(),
        };

        for s in DEFAULT_SYMBOLS {
            this.reg_symbol(String::from(s));
        }

        this
    }
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
                self.locals.insert(k);
                self.scope.insert(k, v);
            }
        } else {
            for k in self.locals.drain() {
                self.scope.remove(&k);
            }
        }
    }

    #[inline(always)]
    fn get(&self, key: &Symbol) -> ZapResult {
        self.scope.get(key)
            .cloned()
            .ok_or_else(|| {
                match self.get_symbol(key) {
                    Ok(s) => error(format!("symbol '{}' not in scope.", s).as_str()),
                    Err(err) => err,
                }
            })
    }

    #[inline(always)]
    fn set(&mut self, key: &ZapExp, val: &ZapExp) -> Result<(), ZapErr> {
        if let ZapExp::Symbol(s) = key {
            if !self.locals.contains(s) {
                self.locals.insert(*s);
            }

            self.scope.insert(*s, val.clone());

            Ok(())
        } else {
            Err(error("Env set: only symbols can be used as keys."))
        }
    }

    fn set_global(&mut self, key: &ZapExp, val: &ZapExp) -> Result<(), ZapErr> {
        if let ZapExp::Symbol(s) = key {
            self.scope.insert(*s, val.clone());
            Ok(())
        } else {
            Err(error("Env set: only symbols can be used as keys."))
        }
    }

    fn reg_symbol(&mut self, s: String) -> ZapExp {
        let len = self.symbols.len();
        let id = self.symbols.entry(s).or_insert(len);
        ZapExp::Symbol(*id)
    }

    fn get_symbol(&self, id: &Symbol) -> Result<String, ZapErr> {
        self.symbols
            .iter()
            .find(|(_, v)| **v == *id)
            .map(|(k, _)| k.clone())
            .ok_or_else(|| error(format!("No known symbol for id={}", *id).as_str()))
    }

    fn reg_fn(&mut self, symbol: &str, f: ZapFnNative) {
        if let ZapExp::Symbol(id) = self.reg_symbol(String::from(symbol)) {
            self.scope
                .insert(id, ZapFn::native(String::from(symbol), f));
        }
    }
}
