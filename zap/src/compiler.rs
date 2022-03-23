use crate::env::{symbols, Env};
use crate::vm::{Chunk, Op};
use crate::zap::{error_msg, Result, Symbol, Value, ZapList};
use std::ops::Range;
use std::rc::Rc;

// The compiler takes the expression returned by the reader and return an array of bytecodes
// which can be executed by the VM.

enum ApplyKind {
    Call,
    Add,
}

enum Form {
    Value(Value),
    List(ZapList, u8),
    Apply(ApplyKind),
}

struct Compiler {
    chunk: Vec<Op>,
    forms: Vec<Form>,
    next_available_reg: Option<u8>,
    argc: u8,
}

impl Compiler {
    pub fn init(ast: Value) -> Self {
        Compiler {
            chunk: Vec::new(),
            forms: vec![Form::Value(ast)],
            next_available_reg: Some(0),
            argc: 0,
        }
    }

    pub fn get_form(&mut self) -> Option<Form> {
        self.forms.pop()
    }

    pub fn chunk(self) -> Chunk {
        Rc::new(self.chunk)
    }

    pub fn set_argc(&mut self, argc: u8) {
        self.argc = argc;
    }

    fn push(&mut self, val: Value) {
        if let Some(dst) = self.next_available_reg {
            self.emit(Op::Set { dst, val });
            self.next_available_reg = Some(dst + 1);
        } else {
            self.emit(Op::Push { val })
        }
    }

    fn is_root_call(&self) -> bool {
        !self.forms.is_empty()
    }

    fn emit(&mut self, op: Op) {
        self.chunk.push(op);
    }

    fn pop_range(&mut self, range: Range<u8>) {
        for dst in range {
            self.emit(Op::Pop { dst });
        }
    }

    pub fn eval_list(&mut self, list: ZapList) -> Result<()> {
        if list.len() > 255 {
            return Err(error_msg(
                "A function cannot have more than 254 parameters.",
            ));
        }
        self.next_available_reg = None;
        // TODO: Check if all elements of the list are atom. If so, set registers instead of
        // pushing up the stack.
        //
        match list[0] {
            Value::Symbol(symbols::PLUS) => {
                self.forms.push(Form::Apply(ApplyKind::Add));
                self.forms.push(Form::List(list, 1));
                return Ok(());
            }
            _ => self.forms.push(Form::Apply(ApplyKind::Call)),
        }
        self.forms.push(Form::List(list, 0));
        Ok(())
    }

    pub fn eval_next_in_list(&mut self, list: ZapList, idx: u8) {
        let item = list[idx as usize].clone();
        self.forms.push(Form::List(list, idx + 1));
        self.forms.push(Form::Value(item));
    }

    pub fn eval_value(&mut self, val: Value) {
        self.push(val);
    }

    pub fn eval_symbol<E: Env>(&mut self, s: Symbol, _env: &mut E) {
        // TODO
        self.push(Value::Symbol(s));
    }

    pub fn apply(&mut self, kind: ApplyKind) {
        let args_stacked = self.next_available_reg.is_none();
        let mut argc = self.argc;

        match kind {
            ApplyKind::Call => {
                // Arguments were pushed on the stack
                if args_stacked {
                    self.pop_range(argc..0_u8)
                }
                self.emit(Op::Call { argc });
            }
            ApplyKind::Add => {
                argc -= 1; // The '+' symbol was not pushed, but was still counted in teh argc
                if argc == 0 {
                    self.emit(Op::Set {
                        dst: 0,
                        val: Value::Number(0.0),
                    });
                } else {
                    if args_stacked {
                        self.emit(Op::Pop { dst: 0 });
                        argc -= 1;
                    }
                    while argc > 0 {
                        if args_stacked {
                            self.emit(Op::Pop { dst: 1 });
                            self.emit(Op::Add { a: 0, b: 1, dst: 0 });
                        } else {
                            self.emit(Op::Add {
                                a: 0,
                                b: argc - 1,
                                dst: 0,
                            });
                        }
                        argc -= 1;
                    }
                }
            }
        }
        if self.is_root_call() {
            self.emit(Op::PushRet);
        }
    }
}

pub fn compile<E: Env>(ast: Value, env: &mut E) -> Result<Chunk> {
    let mut compiler = Compiler::init(ast);

    while let Some(form) = compiler.get_form() {
        match form {
            Form::Value(val) => match val {
                Value::List(list) => {
                    if list.len() > 0 {
                        compiler.eval_list(list)?;
                    } else {
                        compiler.eval_value(Value::List(list))
                    }
                }
                Value::Symbol(s) => compiler.eval_symbol(s, env),
                atom => compiler.eval_value(atom),
            },
            Form::List(list, idx) => {
                if list.len() > idx.into() {
                    compiler.eval_next_in_list(list, idx)
                } else {
                    compiler.set_argc(idx);
                }
            }
            Form::Apply(kind) => {
                compiler.apply(kind);
            }
        }
    }

    Ok(compiler.chunk())
}
