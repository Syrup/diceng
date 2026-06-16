use crate::types::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Token produced by the lexer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Token kinds
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenKind {
    /// Numeric literal (42, 3, 100)
    Number(i32),
    /// Dice notation (d6, 3d6, dF, d%, d{1,2,3})
    Dice(DiceToken),
    /// Identifier (keep, drop, explode, reroll, compound, emphasis, count, etc.)
    Ident(String),
    /// Shorthand modifier (k, d, e, r, ce, c)
    Shorthand(ModifierShorthand),
    /// Arithmetic operator (+, -, *, /)
    Op(BinaryOp),
    /// Comparison operator (>=, <=, ==, !=, <, >)
    CompOp(CountOp),
    /// Left parenthesis
    LParen,
    /// Right parenthesis
    RParen,
    /// Left bracket
    LBrack,
    /// Right bracket
    RBrack,
    /// Left brace
    LBrace,
    /// Right brace
    RBrace,
    /// Comma
    Comma,
    /// Range operator (..)
    DotDot,
    /// End of input
    Eof,
}

/// Dice token details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiceToken {
    /// Standard: count d sides (e.g., 3d6)
    Standard { count: u32, sides: u32 },
    /// Percent: d% or d100
    Percent { count: u32 },
    /// Fate: dF or dF.N
    Fate { count: u32, magnitude: u32 },
    /// Custom: d{1,2,3}
    Custom { count: u32, faces: Vec<i32> },
}

/// Modifier shorthands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModifierShorthand {
    /// k = keep (default: highest)
    Keep,
    /// kh = keep highest
    KeepHigh,
    /// kl = keep lowest
    KeepLow,
    /// d = drop (default: lowest)
    Drop,
    /// dh = drop highest
    DropHigh,
    /// dl = drop lowest
    DropLow,
    /// e = explode
    Explode,
    /// ! = explode (standard RPG notation)
    ExplodeBang,
    /// r = reroll
    Reroll,
    /// ro = reroll once
    RerollOnce,
    /// ce = compound explode
    Compound,
    /// !! = compound explode (standard RPG notation)
    CompoundBang,
    /// c = count
    Count,
    /// cs = count successes
    CountSuccess,
    /// t = target number (count >= N)
    Target,
    /// mi = minimum cap
    MinCap,
    /// ma = maximum cap
    MaxCap,
    /// sa = sort ascending
    SortAsc,
    /// sd = sort descending
    SortDesc,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Number(n) => write!(f, "{}", n),
            TokenKind::Dice(_) => write!(f, "dice"),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Shorthand(s) => write!(f, "{:?}", s),
            TokenKind::Op(op) => write!(f, "{}", op),
            TokenKind::CompOp(op) => write!(f, "{}", op),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrack => write!(f, "["),
            TokenKind::RBrack => write!(f, "]"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

/// Lexer error
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LexError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lexer error at position {}: {}",
            self.position, self.message
        )
    }
}

/// Lexer that tokenizes dice expression strings
pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    prev_was_dice: bool,
    prev_was_filter_number: bool, // true when previous token was Number after Shorthand
    prev_was_shorthand: bool,     // true when previous token was Shorthand
}

impl Lexer {
    /// Create a new lexer for the given input string
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            prev_was_dice: false,
            prev_was_filter_number: false,
            prev_was_shorthand: false,
        }
    }

    /// Tokenize the input string into a list of tokens
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    text: String::new(),
                    start: self.pos,
                    end: self.pos,
                });
                break;
            }

            let start = self.pos;
            let ch = self.current();

            let token = match ch {
                '(' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::LParen,
                        text: "(".into(),
                        start,
                        end: self.pos,
                    }
                }
                ')' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::RParen,
                        text: ")".into(),
                        start,
                        end: self.pos,
                    }
                }
                '[' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::LBrack,
                        text: "[".into(),
                        start,
                        end: self.pos,
                    }
                }
                ']' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::RBrack,
                        text: "]".into(),
                        start,
                        end: self.pos,
                    }
                }
                '{' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::LBrace,
                        text: "{".into(),
                        start,
                        end: self.pos,
                    }
                }
                '}' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::RBrace,
                        text: "}".into(),
                        start,
                        end: self.pos,
                    }
                }
                ',' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::Comma,
                        text: ",".into(),
                        start,
                        end: self.pos,
                    }
                }
                '+' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::Op(BinaryOp::Add),
                        text: "+".into(),
                        start,
                        end: self.pos,
                    }
                }
                '-' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::Op(BinaryOp::Sub),
                        text: "-".into(),
                        start,
                        end: self.pos,
                    }
                }
                '*' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::Op(BinaryOp::Mul),
                        text: "*".into(),
                        start,
                        end: self.pos,
                    }
                }
                '/' => {
                    self.advance();
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = false;
                    Token {
                        kind: TokenKind::Op(BinaryOp::Div),
                        text: "/".into(),
                        start,
                        end: self.pos,
                    }
                }
                '×' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Op(BinaryOp::Mul),
                        text: "×".into(),
                        start,
                        end: self.pos,
                    }
                }
                '⋅' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Op(BinaryOp::Mul),
                        text: "⋅".into(),
                        start,
                        end: self.pos,
                    }
                }
                '÷' => {
                    self.advance();
                    Token {
                        kind: TokenKind::Op(BinaryOp::Div),
                        text: "÷".into(),
                        start,
                        end: self.pos,
                    }
                }
                '.' if self.peek() == Some('.') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::DotDot,
                        text: "..".into(),
                        start,
                        end: self.pos,
                    }
                }
                '>' if self.peek() == Some('=') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Ge),
                        text: ">=".into(),
                        start,
                        end: self.pos,
                    }
                }
                '>' => {
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Gt),
                        text: ">".into(),
                        start,
                        end: self.pos,
                    }
                }
                '<' if self.peek() == Some('=') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Le),
                        text: "<=".into(),
                        start,
                        end: self.pos,
                    }
                }
                '<' => {
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Lt),
                        text: "<".into(),
                        start,
                        end: self.pos,
                    }
                }
                '=' if self.peek() == Some('=') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Eq),
                        text: "==".into(),
                        start,
                        end: self.pos,
                    }
                }
                '!' if self.peek() == Some('!')
                    && self.input.get(self.pos + 2) != Some(&'=')
                    && self.prev_was_dice =>
                {
                    // !! = compound explode (standard RPG notation)
                    self.advance();
                    self.advance();
                    let tok = Token {
                        kind: TokenKind::Shorthand(ModifierShorthand::CompoundBang),
                        text: "!!".into(),
                        start,
                        end: self.pos,
                    };
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = true;
                    tok
                }
                '!' if self.peek() != Some('=') && self.prev_was_dice => {
                    // ! = explode (standard RPG notation)
                    self.advance();
                    let tok = Token {
                        kind: TokenKind::Shorthand(ModifierShorthand::ExplodeBang),
                        text: "!".into(),
                        start,
                        end: self.pos,
                    };
                    self.prev_was_dice = false;
                    self.prev_was_shorthand = true;
                    tok
                }
                '!' if self.peek() == Some('=') => {
                    self.advance();
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Ne),
                        text: "!=".into(),
                        start,
                        end: self.pos,
                    }
                }
                '≤' => {
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Le),
                        text: "≤".into(),
                        start,
                        end: self.pos,
                    }
                }
                '≥' => {
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Ge),
                        text: "≥".into(),
                        start,
                        end: self.pos,
                    }
                }
                '≠' => {
                    self.advance();
                    Token {
                        kind: TokenKind::CompOp(CountOp::Ne),
                        text: "≠".into(),
                        start,
                        end: self.pos,
                    }
                }
                '0'..='9' => {
                    let tok = self.lex_number_or_dice(start)?;
                    let is_dice = matches!(&tok.kind, TokenKind::Dice(_));
                    self.prev_was_dice = is_dice;
                    self.prev_was_shorthand = false;
                    // prev_was_filter_number stays true only if it was set by a Shorthand
                    // (it's already false if no Shorthand preceded this Number)
                    tok
                }
                'd' | 'D' => {
                    if self.prev_was_dice || self.prev_was_filter_number {
                        // After a dice token OR after a filter number (k3d2),
                        // check if 'd' is a shorthand (d1, d3)
                        // or the start of a keyword (drop, drop 1)
                        let next_char = self.peek();
                        let is_shorthand = matches!(next_char, Some(c) if c.is_ascii_digit() || c == ' ')
                            || next_char.is_none();

                        if is_shorthand {
                            // 'd' followed by digit or space = drop shorthand
                            self.advance();
                            let tok = Token {
                                kind: TokenKind::Shorthand(ModifierShorthand::Drop),
                                text: self.input[start..self.pos].iter().collect(),
                                start,
                                end: self.pos,
                            };
                            self.prev_was_dice = false;
                            self.prev_was_filter_number = false;
                            self.prev_was_shorthand = true; // Drop is a Shorthand
                            tok
                        } else {
                            // 'd' followed by letters = identifier (e.g., "drop")
                            let tok = self.lex_ident_or_keyword(start)?;
                            self.prev_was_dice = false;
                            self.prev_was_filter_number = false;
                            // lex_ident_or_keyword will set prev_was_shorthand if needed
                            tok
                        }
                    } else {
                        // Check if 'd' followed by a letter (not digit/%/F/{)
                        // If so, treat as identifier (e.g., "dh", "dl", "drop")
                        let next_char = self.peek();
                        if matches!(next_char, Some(c) if c.is_ascii_alphabetic() && c != 'F' && c != 'f')
                        {
                            let tok = self.lex_ident_or_keyword(start)?;
                            self.prev_was_dice = false;
                            self.prev_was_filter_number = false;
                            let is_shorthand = matches!(&tok.kind, TokenKind::Shorthand(_));
                            self.prev_was_filter_number = is_shorthand;
                            self.prev_was_shorthand = is_shorthand;
                            tok
                        } else {
                            let tok = self.lex_dice(start)?;
                            self.prev_was_dice = matches!(&tok.kind, TokenKind::Dice(_));
                            self.prev_was_filter_number = false;
                            tok
                        }
                    }
                }
                'a'..='z' | 'A'..='Z' | '_' => {
                    let tok = self.lex_ident_or_keyword(start)?;
                    self.prev_was_dice = false;
                    // If this is a Shorthand token, mark that a filter number may follow
                    let is_shorthand = matches!(&tok.kind, TokenKind::Shorthand(_));
                    self.prev_was_filter_number = is_shorthand;
                    self.prev_was_shorthand = is_shorthand;
                    tok
                }
                _ => {
                    return Err(LexError {
                        message: format!("Unexpected character '{}'", ch),
                        position: self.pos,
                    });
                }
            };

            tokens.push(token);
        }
        Ok(tokens)
    }

    fn current(&self) -> char {
        self.input[self.pos]
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                ' ' | '\t' | '\n' | '\r' | '_' => self.advance(),
                _ => break,
            }
        }
    }

    fn lex_number_or_dice(&mut self, start: usize) -> Result<Token, LexError> {
        let num = self.read_number()?;

        // If previous token was Shorthand, this number is a filter argument (e.g., k3)
        // NOT a dice expression. Just return Number token.
        if self.prev_was_shorthand {
            self.prev_was_shorthand = false;
            return Ok(Token {
                kind: TokenKind::Number(num),
                text: self.input[start..self.pos].iter().collect(),
                start,
                end: self.pos,
            });
        }

        // Check if followed by 'd' or 'D' for dice notation
        if self.pos < self.input.len()
            && (self.input[self.pos] == 'd' || self.input[self.pos] == 'D')
        {
            self.advance(); // consume 'd'

            if self.pos >= self.input.len() {
                return Err(LexError {
                    message: "Unexpected end after 'd'".into(),
                    position: self.pos,
                });
            }

            let next = self.current();
            match next {
                '%' => {
                    self.advance();
                    Ok(Token {
                        kind: TokenKind::Dice(DiceToken::Percent { count: num as u32 }),
                        text: self.input[start..self.pos].iter().collect(),
                        start,
                        end: self.pos,
                    })
                }
                'F' | 'f' => {
                    self.advance();
                    let mut magnitude = 1u32;
                    // Check for .N suffix (variable fudge dice)
                    if self.pos < self.input.len()
                        && self.input[self.pos] == '.'
                        && self.pos + 1 < self.input.len()
                        && self.input[self.pos + 1].is_ascii_digit()
                    {
                        self.advance(); // consume '.'
                        let mag = self.read_number()? as u32;
                        if mag >= 2 {
                            magnitude = mag;
                        }
                    }
                    Ok(Token {
                        kind: TokenKind::Dice(DiceToken::Fate {
                            count: num as u32,
                            magnitude,
                        }),
                        text: self.input[start..self.pos].iter().collect(),
                        start,
                        end: self.pos,
                    })
                }
                '{' => {
                    self.advance(); // consume '{'
                    let faces = self.read_face_list()?;
                    if self.pos < self.input.len() && self.current() == '}' {
                        self.advance(); // consume '}'
                    }
                    Ok(Token {
                        kind: TokenKind::Dice(DiceToken::Custom {
                            count: num as u32,
                            faces,
                        }),
                        text: self.input[start..self.pos].iter().collect(),
                        start,
                        end: self.pos,
                    })
                }
                '0'..='9' => {
                    let sides = self.read_number()?;
                    // Check for shorthand modifiers after dice (e.g., 4d6k3)
                    self.try_lex_dice_shorthand(start, num as u32, sides as u32)
                }
                _ => {
                    // d without sides number — treat as d6? No, error.
                    Err(LexError {
                        message: format!(
                            "Expected number, '%', 'F', or '{{' after 'd', got '{}'",
                            next
                        ),
                        position: self.pos,
                    })
                }
            }
        } else {
            // Just a number
            Ok(Token {
                kind: TokenKind::Number(num),
                text: self.input[start..self.pos].iter().collect(),
                start,
                end: self.pos,
            })
        }
    }

    fn try_lex_dice_shorthand(
        &mut self,
        start: usize,
        count: u32,
        sides: u32,
    ) -> Result<Token, LexError> {
        // Don't consume shorthand modifiers (k, d, e, r, ce, c) here.
        // They will be lexed as separate tokens in the next iteration
        // of the main tokenize() loop, which will match them via
        // lex_ident_or_keyword() -> Shorthand tokens.
        Ok(Token {
            kind: TokenKind::Dice(DiceToken::Standard { count, sides }),
            text: self.input[start..self.pos].iter().collect(),
            start,
            end: self.pos,
        })
    }

    fn lex_dice(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance(); // consume 'd'

        if self.pos >= self.input.len() {
            return Err(LexError {
                message: "Unexpected end after 'd'".into(),
                position: self.pos,
            });
        }

        let next = self.current();
        match next {
            '%' => {
                self.advance();
                Ok(Token {
                    kind: TokenKind::Dice(DiceToken::Percent { count: 1 }),
                    text: self.input[start..self.pos].iter().collect(),
                    start,
                    end: self.pos,
                })
            }
            'F' | 'f' => {
                self.advance();
                let mut magnitude = 1u32;
                // Check for .N suffix (variable fudge dice)
                if self.pos < self.input.len()
                    && self.input[self.pos] == '.'
                    && self.pos + 1 < self.input.len()
                    && self.input[self.pos + 1].is_ascii_digit()
                {
                    self.advance(); // consume '.'
                    let mag = self.read_number()? as u32;
                    if mag >= 2 {
                        magnitude = mag;
                    }
                }
                Ok(Token {
                    kind: TokenKind::Dice(DiceToken::Fate {
                        count: 1,
                        magnitude,
                    }),
                    text: self.input[start..self.pos].iter().collect(),
                    start,
                    end: self.pos,
                })
            }
            '{' => {
                self.advance();
                let faces = self.read_face_list()?;
                if self.pos < self.input.len() && self.current() == '}' {
                    self.advance();
                }
                Ok(Token {
                    kind: TokenKind::Dice(DiceToken::Custom { count: 1, faces }),
                    text: self.input[start..self.pos].iter().collect(),
                    start,
                    end: self.pos,
                })
            }
            '0'..='9' => {
                let sides = self.read_number()?;
                if sides == 0 {
                    return Err(LexError {
                        message: "Die cannot have 0 sides".into(),
                        position: start,
                    });
                }
                Ok(Token {
                    kind: TokenKind::Dice(DiceToken::Standard {
                        count: 1,
                        sides: sides as u32,
                    }),
                    text: self.input[start..self.pos].iter().collect(),
                    start,
                    end: self.pos,
                })
            }
            _ => Err(LexError {
                message: format!(
                    "Expected number, '%', 'F', or '{{' after 'd', got '{}'",
                    next
                ),
                position: self.pos,
            }),
        }
    }

    fn lex_ident_or_keyword(&mut self, start: usize) -> Result<Token, LexError> {
        let ident = self.read_ident();

        // Handle shorthand letters followed by digits (e.g., "k3", "d1", "e5", "k3d2")
        // We need to split them: emit the shorthand token, rewind so digits
        // are lexed as a separate Number token in the next iteration.
        if ident.len() > 1 {
            let first_char = ident.chars().next().unwrap();

            // Check for two-letter shorthands FIRST: "ce" (compound)
            if first_char == 'c' && ident.len() > 1 {
                let second_char = ident.chars().nth(1).unwrap();
                if second_char == 'e' {
                    // "ce..." - compound shorthand
                    let ce_digit_len = ident[2..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .count();
                    if ce_digit_len > 0 || ident.len() == 2 {
                        // "ce6" or "ce" - emit Compound shorthand
                        self.pos = start + 2;
                        return Ok(Token {
                            kind: TokenKind::Shorthand(ModifierShorthand::Compound),
                            text: self.input[start..self.pos].iter().collect(),
                            start,
                            end: self.pos,
                        });
                    }
                } else if second_char.is_ascii_digit() {
                    // "c6" - count shorthand
                    let c_digit_len = ident[1..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .count();
                    if c_digit_len > 0 {
                        self.pos = start + 1;
                        return Ok(Token {
                            kind: TokenKind::Shorthand(ModifierShorthand::Count),
                            text: self.input[start..self.pos].iter().collect(),
                            start,
                            end: self.pos,
                        });
                    }
                }
            }

            // Check for two-letter shorthands: kh, kl, dh, dl, mi, ma, ro, cs, sa, sd
            if first_char == 'k' && ident.len() > 1 {
                let second_char = ident.chars().nth(1).unwrap();
                if second_char == 'h' || second_char == 'l' {
                    let digit_len = ident[2..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .count();
                    if digit_len > 0 || ident.len() == 2 {
                        let shorthand = if second_char == 'h' {
                            ModifierShorthand::KeepHigh
                        } else {
                            ModifierShorthand::KeepLow
                        };
                        self.pos = start + 2;
                        return Ok(Token {
                            kind: TokenKind::Shorthand(shorthand),
                            text: self.input[start..self.pos].iter().collect(),
                            start,
                            end: self.pos,
                        });
                    }
                }
            }
            if first_char == 'd' && ident.len() > 1 {
                let second_char = ident.chars().nth(1).unwrap();
                if second_char == 'h' || second_char == 'l' {
                    let digit_len = ident[2..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .count();
                    if digit_len > 0 || ident.len() == 2 {
                        let shorthand = if second_char == 'h' {
                            ModifierShorthand::DropHigh
                        } else {
                            ModifierShorthand::DropLow
                        };
                        self.pos = start + 2;
                        return Ok(Token {
                            kind: TokenKind::Shorthand(shorthand),
                            text: self.input[start..self.pos].iter().collect(),
                            start,
                            end: self.pos,
                        });
                    }
                }
            }
            if first_char == 'm' && ident.len() > 1 {
                let second_char = ident.chars().nth(1).unwrap();
                if second_char == 'i' || second_char == 'a' {
                    let digit_len = ident[2..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .count();
                    if digit_len > 0 || ident.len() == 2 {
                        let shorthand = if second_char == 'i' {
                            ModifierShorthand::MinCap
                        } else {
                            ModifierShorthand::MaxCap
                        };
                        self.pos = start + 2;
                        return Ok(Token {
                            kind: TokenKind::Shorthand(shorthand),
                            text: self.input[start..self.pos].iter().collect(),
                            start,
                            end: self.pos,
                        });
                    }
                }
            }
            if first_char == 'r' && ident.len() > 1 {
                let second_char = ident.chars().nth(1).unwrap();
                if second_char == 'o' {
                    let digit_len = ident[2..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .count();
                    if digit_len > 0 || ident.len() == 2 {
                        self.pos = start + 2;
                        return Ok(Token {
                            kind: TokenKind::Shorthand(ModifierShorthand::RerollOnce),
                            text: self.input[start..self.pos].iter().collect(),
                            start,
                            end: self.pos,
                        });
                    }
                }
            }
            if first_char == 't' && ident.len() > 1 {
                let second_char = ident.chars().nth(1).unwrap();
                if second_char.is_ascii_digit() {
                    self.pos = start + 1;
                    return Ok(Token {
                        kind: TokenKind::Shorthand(ModifierShorthand::Target),
                        text: self.input[start..self.pos].iter().collect(),
                        start,
                        end: self.pos,
                    });
                }
            }
            if first_char == 'c' && ident.len() > 1 {
                let second_char = ident.chars().nth(1).unwrap();
                if second_char == 's' {
                    let digit_len = ident[2..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .count();
                    if digit_len > 0 || ident.len() == 2 {
                        self.pos = start + 2;
                        return Ok(Token {
                            kind: TokenKind::Shorthand(ModifierShorthand::CountSuccess),
                            text: self.input[start..self.pos].iter().collect(),
                            start,
                            end: self.pos,
                        });
                    }
                }
            }

            // Find length of digit prefix after the first character
            let digit_prefix_len = ident[1..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .count();

            // If there are digits after the first char, and first char is a shorthand
            if digit_prefix_len > 0 {
                let shorthand = match first_char {
                    'k' | 'K' => Some(ModifierShorthand::Keep),
                    'd' | 'D' => Some(ModifierShorthand::Drop),
                    'e' | 'E' => Some(ModifierShorthand::Explode),
                    'r' | 'R' => Some(ModifierShorthand::Reroll),
                    _ => None,
                };

                if let Some(sh) = shorthand {
                    // Rewind position to just after the shorthand letter
                    self.pos = start + 1;
                    return Ok(Token {
                        kind: TokenKind::Shorthand(sh),
                        text: self.input[start..self.pos].iter().collect(),
                        start,
                        end: self.pos,
                    });
                }
            }
        }

        // Check for keywords
        let kind = match ident.to_lowercase().as_str() {
            "keep" | "k" => TokenKind::Shorthand(ModifierShorthand::Keep),
            "drop" => TokenKind::Shorthand(ModifierShorthand::Drop),
            "explode" | "e" => TokenKind::Shorthand(ModifierShorthand::Explode),
            "reroll" | "r" => TokenKind::Shorthand(ModifierShorthand::Reroll),
            "compound" | "ce" => TokenKind::Shorthand(ModifierShorthand::Compound),
            "count" | "c" => TokenKind::Shorthand(ModifierShorthand::Count),
            "sum" => TokenKind::Ident("sum".into()),
            "min" | "minimum" | "least" => TokenKind::Ident("min".into()),
            "max" | "maximum" | "best" => TokenKind::Ident("max".into()),
            "average" | "avg" => TokenKind::Ident("average".into()),
            "median" | "med" => TokenKind::Ident("median".into()),
            "highest" | "high" => TokenKind::Ident("highest".into()),
            "lowest" | "low" => TokenKind::Ident("lowest".into()),
            "middle" | "mid" => TokenKind::Ident("middle".into()),
            "on" => TokenKind::Ident("on".into()),
            "or" => TokenKind::Ident("or".into()),
            "more" => TokenKind::Ident("more".into()),
            "less" => TokenKind::Ident("less".into()),
            "than" => TokenKind::Ident("than".into()),
            "once" => TokenKind::Ident("once".into()),
            "twice" => TokenKind::Ident("twice".into()),
            "thrice" => TokenKind::Ident("thrice".into()),
            "times" => TokenKind::Ident("times".into()),
            "always" => TokenKind::Ident("always".into()),
            "take" => TokenKind::Ident("take".into()),
            "emphasis" => TokenKind::Ident("emphasis".into()),
            "furthest" => TokenKind::Ident("furthest".into()),
            "from" => TokenKind::Ident("from".into()),
            "and" => TokenKind::Ident("and".into()),
            "exactly" => TokenKind::Ident("exactly".into()),
            "cs" | "csuccess" => TokenKind::Shorthand(ModifierShorthand::CountSuccess),
            "t" => TokenKind::Shorthand(ModifierShorthand::Target),
            "sa" => TokenKind::Shorthand(ModifierShorthand::SortAsc),
            "sd" => TokenKind::Shorthand(ModifierShorthand::SortDesc),
            _ => TokenKind::Ident(ident.clone()),
        };

        Ok(Token {
            kind,
            text: self.input[start..self.pos].iter().collect(),
            start,
            end: self.pos,
        })
    }

    fn read_number(&mut self) -> Result<i32, LexError> {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.advance();
        }
        let s: String = self.input[start..self.pos].iter().collect();
        s.parse::<i32>().map_err(|_| LexError {
            message: format!("Number '{}' is out of range", s),
            position: start,
        })
    }

    fn read_ident(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => self.advance(),
                _ => break,
            }
        }
        self.input[start..self.pos].iter().collect()
    }

    fn read_face_list(&mut self) -> Result<Vec<i32>, LexError> {
        let mut faces = Vec::new();
        loop {
            self.skip_whitespace();
            if self.pos >= self.input.len() || self.current() == '}' {
                break;
            }
            // Handle optional negative sign
            let negative = if self.current() == '-' {
                self.advance();
                true
            } else {
                false
            };
            let num = self.read_number()?;
            faces.push(if negative { -num } else { num });
            self.skip_whitespace();
            if self.pos < self.input.len() && self.current() == ',' {
                self.advance();
            }
        }
        if faces.is_empty() {
            return Err(LexError {
                message: "Empty face list".into(),
                position: self.pos,
            });
        }
        Ok(faces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_dice() {
        let tokens = Lexer::new("3d6").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 3, sides: 6 })
        );
    }

    #[test]
    fn test_d6() {
        let tokens = Lexer::new("d6").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 1, sides: 6 })
        );
    }

    #[test]
    fn test_fate_dice() {
        let tokens = Lexer::new("dF").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Fate {
                count: 1,
                magnitude: 1
            })
        );
    }

    #[test]
    fn test_percent() {
        let tokens = Lexer::new("d%").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Percent { count: 1 })
        );
    }

    #[test]
    fn test_custom_dice() {
        let tokens = Lexer::new("d{1,2,3}").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Custom {
                count: 1,
                faces: vec![1, 2, 3]
            })
        );
    }

    #[test]
    fn test_arithmetic() {
        let tokens = Lexer::new("3d6+4").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 3, sides: 6 })
        );
        assert_eq!(tokens[1].kind, TokenKind::Op(BinaryOp::Add));
        assert_eq!(tokens[2].kind, TokenKind::Number(4));
    }

    #[test]
    fn test_keep_shorthand() {
        let tokens = Lexer::new("4d6k3").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::Keep)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(3));
    }

    #[test]
    fn test_keep_shorthand_10d6k3() {
        let tokens = Lexer::new("10d6k3").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard {
                count: 10,
                sides: 6
            })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::Keep)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(3));
    }

    #[test]
    fn test_drop_shorthand() {
        let tokens = Lexer::new("4d6d1").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::Drop)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(1));
    }

    #[test]
    fn test_explode_shorthand() {
        let tokens = Lexer::new("3d6e5").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 3, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::Explode)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(5));
    }

    #[test]
    fn test_reroll_shorthand() {
        let tokens = Lexer::new("3d6r2").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 3, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::Reroll)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(2));
    }

    #[test]
    fn test_keep_shorthand_no_number() {
        let tokens = Lexer::new("4d6k").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::Keep)
        );
    }

    #[test]
    fn test_chained_shorthand_k3d2() {
        let tokens = Lexer::new("20d6k3d2").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard {
                count: 20,
                sides: 6
            })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::Keep)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(3));
        assert_eq!(
            tokens[3].kind,
            TokenKind::Shorthand(ModifierShorthand::Drop)
        );
        assert_eq!(tokens[4].kind, TokenKind::Number(2));
    }

    // Tests for new standard RPG notation features

    #[test]
    fn test_bang_explode() {
        let tokens = Lexer::new("3d6!").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 3, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::ExplodeBang)
        );
    }

    #[test]
    fn test_bang_explode_with_condition() {
        let tokens = Lexer::new("3d6!>=5").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 3, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::ExplodeBang)
        );
        assert_eq!(tokens[2].kind, TokenKind::CompOp(CountOp::Ge));
        assert_eq!(tokens[3].kind, TokenKind::Number(5));
    }

    #[test]
    fn test_bang_compound() {
        let tokens = Lexer::new("3d6!!").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 3, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::CompoundBang)
        );
    }

    #[test]
    fn test_keep_high_shorthand() {
        let tokens = Lexer::new("4d6kh3").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::KeepHigh)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(3));
    }

    #[test]
    fn test_keep_low_shorthand() {
        let tokens = Lexer::new("4d6kl1").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::KeepLow)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(1));
    }

    #[test]
    fn test_drop_high_shorthand() {
        let tokens = Lexer::new("4d6dh1").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::DropHigh)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(1));
    }

    #[test]
    fn test_drop_low_shorthand() {
        let tokens = Lexer::new("4d6dl1").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::DropLow)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(1));
    }

    #[test]
    fn test_reroll_once_shorthand() {
        let tokens = Lexer::new("2d6ro1").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 2, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::RerollOnce)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(1));
    }

    #[test]
    fn test_count_success_keyword() {
        let tokens = Lexer::new("4d6cs>=4").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::CountSuccess)
        );
        assert_eq!(tokens[2].kind, TokenKind::CompOp(CountOp::Ge));
        assert_eq!(tokens[3].kind, TokenKind::Number(4));
    }

    #[test]
    fn test_target_shorthand() {
        let tokens = Lexer::new("4d6t4").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::Target)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(4));
    }

    #[test]
    fn test_min_cap_shorthand() {
        let tokens = Lexer::new("4d6mi2").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::MinCap)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(2));
    }

    #[test]
    fn test_max_cap_shorthand() {
        let tokens = Lexer::new("4d6ma5").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::MaxCap)
        );
        assert_eq!(tokens[2].kind, TokenKind::Number(5));
    }

    #[test]
    fn test_sort_ascending() {
        let tokens = Lexer::new("4d6sa").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::SortAsc)
        );
    }

    #[test]
    fn test_sort_descending() {
        let tokens = Lexer::new("4d6sd").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Standard { count: 4, sides: 6 })
        );
        assert_eq!(
            tokens[1].kind,
            TokenKind::Shorthand(ModifierShorthand::SortDesc)
        );
    }

    #[test]
    fn test_variable_fudge_dice() {
        let tokens = Lexer::new("dF.2").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Fate {
                count: 1,
                magnitude: 2
            })
        );
    }

    #[test]
    fn test_variable_fudge_dice_with_count() {
        let tokens = Lexer::new("4dF.3").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Fate {
                count: 4,
                magnitude: 3
            })
        );
    }

    // ── Error Path Tests ──────────────────────────────────────────────

    #[test]
    fn test_lex_error_unexpected_char() {
        let result = Lexer::new("3d6@#").tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unexpected character"));
        assert_eq!(err.position, 3);
    }

    #[test]
    fn test_lex_error_d0() {
        let result = Lexer::new("d0").tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("0 sides"));
    }

    #[test]
    fn test_lex_error_empty_faces() {
        let result = Lexer::new("d{}").tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Empty face list"));
    }

    #[test]
    fn test_lex_error_unterminated_d() {
        let result = Lexer::new("3d").tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unexpected end after 'd'"));
    }

    #[test]
    fn test_lex_empty_input() {
        let tokens = Lexer::new("").tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_whitespace_only() {
        let tokens = Lexer::new("   ").tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_lex_d_followed_by_letter_is_identifier() {
        // "dabc" is a valid identifier (not dice notation)
        let tokens = Lexer::new("dabc").tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Ident("dabc".into()));
    }

    #[test]
    fn test_lex_negative_faces() {
        let tokens = Lexer::new("d{-1,0,1}").tokenize().unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::Dice(DiceToken::Custom {
                count: 1,
                faces: vec![-1, 0, 1]
            })
        );
    }
}
