use crate::zap::{error_msg, Result, String, Symbol, Value, ZapFnNative};
use fxhash::FxHashMap;

pub type Scope = Vec<Option<Value>>;
pub type SymbolTable = FxHashMap<String, Symbol>;

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
    fn get_by_id(&self, id: Symbol) -> Result<Value>;
    fn set(&mut self, key: &Value, val: &Value) -> Result<()>;
    fn reg_symbol(&mut self, s: String) -> Value;
    fn get_symbol(&self, key: Symbol) -> Result<String>;

    fn reg_fn(&mut self, symbol: &str, f: fn(&[Value]) -> Result<Value>) -> Result<()> {
        let id = self.reg_symbol(String::from(symbol));
        self.set(
            &id,
            &Value::FuncNative(ZapFnNative::new(String::from(symbol), f)),
        )?;
        Ok(())
    }

    #[inline(always)]
    fn get(&self, key: &Value) -> Result<Value> {
        match key {
            Value::Symbol(id) => self.get_by_id(*id),
            _ => Err(error_msg("Only symbols can be used as keys in env.")),
        }
    }
}

pub struct SandboxEnv {
    globals: Scope,
    symbols: SymbolTable,
}

impl Default for SandboxEnv {
    fn default() -> Self {
        let mut this = SandboxEnv {
            globals: Scope::default(),
            symbols: SymbolTable::default(),
        };

        for s in symbols::DEFAULT_SYMBOLS {
            this.reg_symbol(String::from(s));
        }

        this
    }
}

impl Env for SandboxEnv {
    #[inline(always)]
    fn get_by_id(&self, id: Symbol) -> Result<Value> {
        match unsafe { &self.globals.get_unchecked(id as usize) } {
            Some(val) => Ok(val.clone()),
            None => Err(match self.get_symbol(id) {
                Ok(s) => error_msg(format!("symbol '{}' not in scope.", s).as_str()),
                Err(err) => err,
            }),
        }
    }

    fn set(&mut self, key: &Value, val: &Value) -> Result<()> {
        if let Value::Symbol(s) = key {
            self.globals[*s as usize] = Some(val.clone());
            Ok(())
        } else {
            Err(error_msg("Env set: only symbols can be used as keys."))
        }
    }

    fn reg_symbol(&mut self, s: String) -> Value {
        let len = self.symbols.len();
        let id = self.symbols.entry(s).or_insert_with(|| {
            self.globals.push(None);
            len.try_into().unwrap()
        });
        Value::Symbol(*id)
    }

    fn get_symbol(&self, id: Symbol) -> Result<String> {
        self.symbols
            .iter()
            .find(|(_, v)| **v == id)
            .map(|(k, _)| k.clone())
            .ok_or_else(|| error_msg(format!("No known symbol for id={}", id).as_str()))
    }
}
