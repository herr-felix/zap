use std::collections::VecDeque;
use std::iter::Peekable;
use std::num::ParseFloatError;
use std::str::Chars;

use crate::types::{error, ZapErr, ZapExp};

/* Tokenizer */

#[derive(PartialEq)]
enum Token {
    Atom(String),
    Quote,
    Unquote,
    ListStart,
    ListEnd,
    SpliceUnquote,
    Deref,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Atom(s) => write!(f, "Atom({})", s),
            Token::Quote => write!(f, "Quote"),
            Token::Unquote => write!(f, "Unquote"),
            Token::SpliceUnquote => write!(f, "SpliceUnquote"),
            Token::Deref => write!(f, "Deref"),
            Token::ListStart => write!(f, "ListStart"),
            Token::ListEnd => write!(f, "ListEnd"),
        }
    }
}

enum ParentForm {
    List(Vec<ZapExp>),
    Quote,
    Unquote,
    SpliceUnquote,
    Deref,
}

pub struct Reader {
    tokens: VecDeque<Token>,
    token_buf: String,
    stack: Vec<ParentForm>,
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

impl Reader {
    pub fn new() -> Reader {
        Reader {
            tokens: VecDeque::new(),
            token_buf: String::with_capacity(32),
            stack: Vec::with_capacity(64),
        }
    }

    fn tokenize_string(&mut self, chars: &mut Peekable<Chars>) {
        let mut escaped = self.token_buf.ends_with('\\');

        #[allow(clippy::while_let_on_iterator)]
        while let Some(ch) = chars.next() {
            if escaped {
                match ch {
                    'n' => self.token_buf.push('\n'),
                    'r' => self.token_buf.push('\r'),
                    '0' => self.token_buf.push('\0'),
                    't' => self.token_buf.push('\t'),
                    _ => self.token_buf.push(ch),
                }
                escaped = false;
            } else {
                match ch {
                    '"' => {
                        self.flush_token();
                        break;
                    }
                    '\\' => {
                        escaped = true;
                        continue;
                    }
                    _ => self.token_buf.push(ch),
                }
            }
        }
    }

    #[inline(always)]
    fn flush_token(&mut self) {
        if !self.token_buf.is_empty() {
            self.token_buf.shrink_to_fit();
            self.tokens.push_back(Token::Atom(self.token_buf.clone()));
            self.token_buf.truncate(0);
        }
    }

    pub fn tokenize(&mut self, src: &str) {
        let mut chars = src.chars().peekable();

        // If the last tokenize call ended while in a string, the token_buf will start if a ", so we
        // want to continue reading that string
        if self.token_buf.starts_with('"') {
            self.tokenize_string(&mut chars);
        }
        // If the last tokenize call ended in a comment
        else if self.token_buf.starts_with(';') {
            if chars.any(|ch| ch == '\n') {
                self.token_buf.truncate(0);
            }
        } else if self.token_buf.starts_with('~') {
            match chars.peek() {
                Some('@') => {
                    chars.next();
                    self.tokens.push_back(Token::SpliceUnquote);
                }
                Some(_) => {
                    self.tokens.push_back(Token::Unquote);
                    self.token_buf.truncate(0);
                }
                None => {}
            }
        }

        #[allow(clippy::while_let_on_iterator)]
        while let Some(ch) = chars.next() {
            match ch {
                ' ' | '\n' | '\t' | ',' => {
                    self.flush_token();
                }
                '(' => {
                    self.flush_token();
                    self.tokens.push_back(Token::ListStart);
                }
                ')' => {
                    self.flush_token();
                    self.tokens.push_back(Token::ListEnd);
                }
                '\'' => {
                    self.flush_token();
                    self.tokens.push_back(Token::Quote);
                }
                '@' => {
                    self.tokens.push_back(Token::Deref);
                }
                '`' | '^' => {
                    if self.token_buf.is_empty() {
                        self.tokens.push_back(Token::Atom(ch.to_string()));
                    } else {
                        self.token_buf.push(ch);
                    }
                }
                '~' => {
                    if self.token_buf.is_empty() {
                        match chars.peek() {
                            Some('@') => {
                                chars.next();
                                self.tokens.push_back(Token::SpliceUnquote);
                            }
                            Some(_) => self.tokens.push_back(Token::Unquote),
                            None => {
                                self.token_buf.push(ch);
                                break;
                            }
                        }
                    } else {
                        self.token_buf.push(ch);
                    }
                }
                ';' => {
                    self.flush_token();
                    self.token_buf.push(';');
                    if chars.any(|ch| ch == '\n') {
                        self.token_buf.truncate(0);
                    }
                }
                '"' => {
                    self.flush_token();
                    self.token_buf.push('"');
                    self.tokenize_string(&mut chars);
                }
                _ => {
                    self.token_buf.push(ch);
                }
            }
        }
    }

    fn read_atom(mut atom: String) -> ZapExp {
        match atom.as_ref() {
            "nil" => ZapExp::Nil,
            "true" => ZapExp::Bool(true),
            "false" => ZapExp::Bool(false),
            _ => {
                if atom.starts_with('"') {
                    return ZapExp::Str(atom.split_off(1));
                }

                let potential_float: Result<f64, ParseFloatError> = atom.parse();
                match potential_float {
                    Ok(v) => ZapExp::Number(v),
                    Err(_) => ZapExp::Symbol(atom),
                }
            }
        }
    }

    fn read_error(&mut self, msg: &str) -> ZapErr {
        self.stack.truncate(0);
        error(msg)
    }

    #[inline(always)]
    fn expand_reader_macro(&mut self, expanded: &str, exp: ZapExp) {
        self.tokens.push_front(Token::ListEnd);
        self.stack.push(ParentForm::List(vec![
            ZapExp::Symbol(expanded.to_string()),
            exp,
        ]));
    }

    pub fn read_form(&mut self) -> Result<Option<ZapExp>, ZapErr> {
        while let Some(token) = self.tokens.pop_front() {
            let exp = match token {
                Token::Atom(s) => Reader::read_atom(s),
                Token::Quote => {
                    self.stack.push(ParentForm::Quote);
                    continue;
                }
                Token::SpliceUnquote => {
                    self.stack.push(ParentForm::SpliceUnquote);
                    continue;
                }
                Token::Unquote => {
                    self.stack.push(ParentForm::Unquote);
                    continue;
                }
                Token::Deref => {
                    self.stack.push(ParentForm::Deref);
                    continue;
                }
                Token::ListStart => {
                    self.stack.push(ParentForm::List(Vec::new()));
                    continue;
                }
                Token::ListEnd => match self.stack.pop() {
                    Some(ParentForm::List(seq)) => ZapExp::List(seq),
                    Some(ParentForm::Quote) => return Err(self.read_error("Cannot quote a ')'")),
                    Some(ParentForm::Unquote) => {
                        return Err(self.read_error("Cannot unquote a ')'"))
                    }
                    Some(ParentForm::SpliceUnquote) => {
                        return Err(self.read_error("Cannot splice-unquote a ')'"))
                    }
                    Some(ParentForm::Deref) => return Err(self.read_error("Cannot deref a ')'")),
                    None => return Err(self.read_error("A form cannot begin with ')'")),
                },
            };

            match self.stack.pop() {
                Some(ParentForm::List(mut parent)) => {
                    parent.push(exp);
                    self.stack.push(ParentForm::List(parent));
                }
                Some(ParentForm::Quote) => self.expand_reader_macro("quote", exp),
                Some(ParentForm::Unquote) => self.expand_reader_macro("unquote", exp),
                Some(ParentForm::SpliceUnquote) => self.expand_reader_macro("splice-unquote", exp),
                Some(ParentForm::Deref) => self.expand_reader_macro("deref", exp),
                None => return Ok(Some(exp)),
            }
        }

        Ok(None)
    }
}
