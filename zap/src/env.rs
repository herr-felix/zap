use crate::zap::{error_msg, Result, String, Symbol, Value, ZapFn, ZapFnNative};
use fxhash::{FxHashMap, FxHashSet};

type Scope = FxHashMap<Symbol, Value>;
type SymbolTable = FxHashMap<String, Symbol>;

// TODO: Make sures all the default symbols (for special forms) are here.
// TODO: Make a macro that generate const Symbol for each default symbols.
const DEFAULT_SYMBOLS: [&str; 10] = [
    "if",
    "let",
    "fn",
    "do",
    "define",
    "quote",
    "quasiquote",
    "unquote",
    "splice-unquote",
    "+",
];
pub mod symbols {
    use crate::zap::Symbol;

    pub const IF: Symbol = 0;
    pub const LET: Symbol = 1;
    pub const FN: Symbol = 2;
    pub const DO: Symbol = 3;
    pub const DEFINE: Symbol = 4;
    pub const QUOTE: Symbol = 5;
    pub const QUASIQUOTE: Symbol = 6;
    pub const UNQUOTE: Symbol = 7;
    pub const SPLICE_UNQUOTE: Symbol = 8;
    pub const PLUS: Symbol = 9;
}

pub trait Env {
    fn push(&mut self);
    fn pop(&mut self);
    fn get(&self, symbol: Symbol) -> Result<Value>;
    fn set(&mut self, key: Symbol, val: &Value) -> Result<()>;
    fn set_global(&mut self, key: &Value, val: &Value) -> Result<()>;
    fn reg_symbol(&mut self, s: String) -> Value;
    fn get_symbol(&self, key: Symbol) -> Result<String>;
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
    fn get(&self, key: Symbol) -> Result<Value> {
        match self.scope.get(&key) {
            Some(val) => Ok(val.clone()),
            None => Err(match self.get_symbol(key) {
                Ok(s) => error_msg(format!("symbol '{}' not in scope.", s).as_str()),
                Err(err) => err,
            }),
        }
    }

    #[inline(always)]
    fn set(&mut self, key: Symbol, val: &Value) -> Result<()> {
        self.locals.insert(key);
        self.scope.insert(key, val.clone());

        Ok(())
    }

    fn set_global(&mut self, key: &Value, val: &Value) -> Result<()> {
        if let Value::Symbol(s) = key {
            self.scope.insert(*s, val.clone());
            Ok(())
        } else {
            Err(error_msg("Env set: only symbols can be used as keys."))
        }
    }

    fn reg_symbol(&mut self, s: String) -> Value {
        let len = self.symbols.len();
        let id = self.symbols.entry(s).or_insert(len);
        Value::Symbol(*id)
    }

    fn get_symbol(&self, id: Symbol) -> Result<String> {
        self.symbols
            .iter()
            .find(|(_, v)| **v == id)
            .map(|(k, _)| k.clone())
            .ok_or_else(|| error_msg(format!("No known symbol for id={}", id).as_str()))
    }

    fn reg_fn(&mut self, symbol: &str, f: ZapFnNative) {
        if let Value::Symbol(id) = self.reg_symbol(String::from(symbol)) {
            self.scope
                .insert(id, ZapFn::native(String::from(symbol), f));
        }
    }
}
