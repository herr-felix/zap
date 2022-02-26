use std::num::ParseFloatError;
use std::str::Chars;

use crate::types::{ZapExp, ZapErr};

/* Tokenizer */

// NOT MY CODE
pub struct Reader {
    pub tokens: Vec<String>,
    pub pos: usize,
}

impl Reader {
    pub fn next(&mut self) -> Option<String> {
        self.pos += 1;
        self.tokens.get(self.pos - 1).and_then(|x| Some(x.to_string()))
    }
    pub fn peek(&self) -> Option<String> {
        self.tokens.get(self.pos).and_then(|x| Some(x.to_string()))
    }
}

// MY CODE
fn tokenize_string(chars: &mut Chars) -> String {
    let mut token = String::from('"');
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if escaped {
            match ch {
                'n' => token.push('\n'),
                'r' => token.push('\r'),
                '0' => token.push('\0'),
                't' => token.push('\t'),
                _ => token.push(ch),
            }
            escaped = false;
        } else {
            match ch {
                '"' => {
                    token.push('"');
                    break;
                }
                '\\' => {
                    escaped = true;
                    continue;
                }
                _ => token.push(ch),
            }
        }
    }
    token
}

fn ignore_line(chars: &mut Chars) {
    for ch in chars {
        if ch == '\n' {
            break
        }
    }
}

#[inline(always)]
fn flush_token(mut token: String, tokens: &mut Vec<String>) -> String {
    if token.len() > 0 {
        tokens.push(token);
        token = String::new();
    }
    return token
}

pub fn tokenize(src: String) -> Vec<String> {
    let mut tokens = Vec::with_capacity(256);
    let mut chars = src.chars();
    let mut token = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '(' | ')'  => {
                token = flush_token(token, &mut tokens);
                tokens.push(ch.to_string());
            },
            '\'' | '`' | '@' | '^' => {
                if token.is_empty() {
                    tokens.push(ch.to_string());
                } else {
                    token.push(ch);
                }
            },
            '~' => {
                if token.is_empty() {
                    match chars.next() {
                        Some('@') => tokens.push("~@".to_string()),
                        Some(ch) => {
                            tokens.push('~'.to_string());
                            token.push(ch);
                        }
                        None => break,
                    }
                } else {
                    token.push(ch);
                }
            },
            ' ' | '\n' | '\t' | ',' => {
                token = flush_token(token, &mut tokens);
            },
            ';' => {
                ignore_line(&mut chars);
                token = flush_token(token, &mut tokens);
            },
            '"' => token = tokenize_string(&mut chars),
            _ => {
                token.push(ch);
            }
        }
    }

    flush_token(token, &mut tokens);

    tokens
}

/* Parser */

fn read_atom(token: &str) -> ZapExp {

    match token.as_ref() {
        "nil" => ZapExp::Nil,
        "true" => ZapExp::Bool(true),
        "false" => ZapExp::Bool(false),
        _ => {
            if token.starts_with('"') && token.ends_with('"') {
                return ZapExp::Str(
                    token
                        .trim_start_matches('"')
                        .trim_end_matches('"')
                        .to_string(),
                );
            }
            let potential_float: Result<f64, ParseFloatError> = token.parse();
            match potential_float {
                Ok(v) => ZapExp::Number(v),
                Err(_) => ZapExp::Symbol(token.to_string()),
            }
        }
    }
}

fn read_seq(rdr: &mut Reader, end: &str) -> Result<ZapExp, ZapErr> {
    let mut seq: Vec<ZapExp> = Vec::new();

    loop {
        if let Some(token) = rdr.peek() {
            if token == end {
                break;
            }
            seq.push(read_form(rdr)?);
        } else {
            return Err(ZapErr::Msg("Unexpected EOF in read_seq".to_string()))
        }
    }

    rdr.next();

    match end {
        ")" => Ok(ZapExp::List(seq)),
        _ => Err(ZapErr::Msg("Unexpected EOF in read_seq".to_string()))
    }
}

pub fn read_form(rdr: &mut Reader) -> Result<ZapExp, ZapErr> {
    if let Some(token) = rdr.next() {
        return match token.as_ref() {
            ")" => Err(ZapErr::Msg("Unexpected ')'".to_string())),
            "(" => read_seq(rdr, ")"),
            _ => Ok(read_atom(token.as_ref())),
        }
    }

    return Err(ZapErr::Msg("Unexpected EOF in read_form".to_string()))
}

