use std::num::ParseFloatError;
use std::str::Chars;
use std::collections::VecDeque;

use crate::types::{ZapExp, ZapErr};

/* Tokenizer */

#[derive(PartialEq)]
enum Token {
    Atom(String),
    Quote,
    ListStart,
    ListEnd,
}

pub struct Reader {
    tokens: VecDeque<Token>,
    token_buf: String,
    stack: Vec<(Vec<ZapExp>, Token)>,
}

impl Reader {

    pub fn new() -> Reader {
        Reader{
            tokens: VecDeque::new(),
            token_buf: String::with_capacity(32),
            stack: Vec::with_capacity(64),
        }        
    }

    fn tokenize_string(&mut self, chars: &mut Chars) {
        let mut escaped = self.token_buf.ends_with('\\');

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
        if self.token_buf.len() > 0 {
            self.token_buf.shrink_to_fit();
            self.tokens.push_back(Token::Atom(self.token_buf.clone()));
            self.token_buf.truncate(0);
        }
    }

    pub fn tokenize(&mut self, src: &str) {
        let mut chars = src.chars();

        // If the last tokenize call ended while in a string, the token_buf will start if a ", so we
        // want to continue reading that string
        if self.token_buf.starts_with('"') {
            self.tokenize_string(&mut chars);
        }
        // If the last tokenize call ended in a comment
        else if self.token_buf.starts_with(";") {  
            if chars.find(|&ch| ch == '\n').is_some() {
                self.token_buf.truncate(0);
            }
        }

        while let Some(ch) = chars.next() {
            match ch {
                ' ' | '\n' | '\t' | ',' => {
                    self.flush_token();
                },
                '(' => {
                    self.flush_token();
                    self.tokens.push_back(Token::ListStart);
                },
                ')' => {
                    self.flush_token();
                    self.tokens.push_back(Token::ListEnd);
                },
                '\'' => {
                    self.flush_token();
                    self.tokens.push_back(Token::Quote);
                },
                '`' | '@' | '^' => {
                    if self.token_buf.is_empty() {
                        self.tokens.push_back(Token::Atom(ch.to_string()));
                    } else {
                        self.token_buf.push(ch);
                    }
                },
                '~' => {
                    if self.token_buf.is_empty() {
                        match chars.next() {
                            Some('@') => self.tokens.push_back(Token::Atom("~@".to_string())),
                            Some(ch) => {
                                self.tokens.push_back(Token::Atom('~'.to_string()));
                                self.token_buf.push(ch);
                            }
                            None => break,
                        }
                    } else {
                        self.token_buf.push(ch);
                    }
                },
                ';' => {
                    self.flush_token();
                    self.token_buf.push(';');
                    if chars.find(|&ch| ch == '\n').is_some() {
                        self.token_buf.truncate(0);
                    }
                },
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
                    return ZapExp::Str(atom.split_off(1))
                }

                let potential_float: Result<f64, ParseFloatError> = atom.parse();
                match potential_float {
                    Ok(v) => ZapExp::Number(v),
                    Err(_) => ZapExp::Symbol(atom),
                }
            }
        }
    }

    pub fn read_form(&mut self) -> Result<Option<ZapExp>, ZapErr> {

        let mut head = self.stack.pop();

        while let Some(token) = self.tokens.pop_front() {
            if let Some((mut seq, end)) = head {
                match token {
                    Token::Atom(s) => {
                        seq.push(Reader::read_atom(s));
                        head = Some((seq, end));
                    },
                    Token::Quote => {
                        // Fuck ca va prendre de la recursion...
                        head = Some((seq, end));
                    },
                    Token::ListStart => {
                        self.stack.push((seq, end));
                        head = Some((Vec::new(), Token::ListEnd));
                    },
                    Token::ListEnd => {
                        if end != token {
                            return Err(ZapErr::Msg("Was not expecting a ')'".to_string()))
                        }
                        // There is a parent
                        if let Some((mut parent, end)) = self.stack.pop() {
                            parent.push(ZapExp::List(seq));
                            head = Some((parent, end));
                        } else {
                            return Ok(Some(ZapExp::List(seq)))
                        }
                    }
                }
            } else {
                match token {
                    Token::Atom(s) => {
                        return Ok(Some(Reader::read_atom(s)))
                    },
                    Token::Quote => {
                        // Fuck ca va prendre de la recursion...
                    },
                    Token::ListStart => {
                        head = Some((Vec::new(), Token::ListEnd));
                    },
                    Token::ListEnd => {
                        return Err(ZapErr::Msg("A form cannot begin with ')'".to_string()))
                    }
                }
            }
        }

        if let Some(seq) = head {
            self.stack.push(seq);
        }

        Ok(None)
    }
}

