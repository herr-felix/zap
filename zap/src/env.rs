use crate::zap::{error_msg, Result, String, Symbol, Value, ZapFn};
use fxhash::FxHashMap;

pub type Scope = FxHashMap<Symbol, Value>;
type SymbolTable = FxHashMap<String, Symbol>;

pub mod symbols {
    use crate::zap::Symbol;
    //
    // TODO: Make sures all the default symbols (for special forms) are here.
    // TODO: Make a macro that generate const Symbol for each default symbols.
    pub const DEFAULT_SYMBOLS: [&str; 11] = [
        "if",
        "let",
        "fn",
        "do",
        "def",
        "quote",
        "quasiquote",
        "unquote",
        "splice-unquote",
        "+",
        "=",
    ];

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
    pub const EQUAL: Symbol = 10;
}

pub trait Env {
    fn get(&self, key: &Value) -> Result<Value>;
    fn set(&mut self, key: &Value, val: &Value) -> Result<()>;
    fn reg_symbol(&mut self, s: String) -> Value;
    fn get_symbol(&self, key: Symbol) -> Result<String>;
    fn reg_fn(&mut self, symbol: &str, f: fn(&[Value]) -> Result<Value>);
}

pub struct SandboxEnv {
    scope: Scope,
    symbols: SymbolTable,
}

impl Default for SandboxEnv {
    fn default() -> Self {
        let mut this = SandboxEnv {
            scope: Scope::default(),
            symbols: SymbolTable::default(),
        };

        for s in symbols::DEFAULT_SYMBOLS {
            this.reg_symbol(String::from(s));
        }

        this
    }
}

impl Env for SandboxEnv {
    fn get(&self, key: &Value) -> Result<Value> {
        match key {
            Value::Symbol(id) => match self.scope.get(id) {
                Some(val) => Ok(val.clone()),
                None => Err(match self.get_symbol(*id) {
                    Ok(s) => error_msg(format!("symbol '{}' not in scope.", s).as_str()),
                    Err(err) => err,
                }),
            },
            _ => Err(error_msg("Only symbols can be used as keys in env.")),
        }
    }

    fn set(&mut self, key: &Value, val: &Value) -> Result<()> {
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

    fn reg_fn(&mut self, symbol: &str, f: fn(&[Value]) -> Result<Value>) {
        if let Value::Symbol(id) = self.reg_symbol(String::from(symbol)) {
            self.scope
                .insert(id, ZapFn::native(String::from(symbol), f));
        }
    }
}
