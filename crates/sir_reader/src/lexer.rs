use thiserror::Error;

use crate::token::Token;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{line}:{column}: {kind}")]
pub struct LexError {
    pub line: usize,
    pub column: usize,

    #[source]
    pub kind: LexErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LexErrorKind {
    #[error("unexpected character '{0}'")]
    UnexpectedCharacter(char),

    #[error("expected identifier or number after '{0}'")]
    ExpectedReference(char),

    #[error("invalid reference")]
    InvalidReference,

    #[error("expected integer digits")]
    ExpectedIntegerDigits,

    #[error("integer literal out of range")]
    IntegerOutOfRange,

    #[error("invalid float literal")]
    InvalidFloat,

    #[error("expected exponent digits")]
    ExpectedExponentDigits,
}

pub struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut chars = self.source[self.pos..].chars();
        chars.next()?;
        chars.next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();

        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(ch)
    }

    fn error(&self, kind: LexErrorKind) -> LexError {
        LexError {
            line: self.line,
            column: self.column,
            kind
        }
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.advance();
        }
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn identifier(&mut self) -> Token {
        let start = self.pos;

        while self
            .peek()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            self.advance();
        }

        let text = &self.source[start..self.pos];

        match text {
            "function" => Token::Function,
            "ldarg" => Token::Ldarg,
            "ldc" => Token::Ldc,
            "ldloc" => Token::Ldloc,
            "stloc" => Token::Stloc,

            "iadd" => Token::Iadd,
            "isub" => Token::Isub,
            "imul" => Token::Imul,
            "idiv" => Token::Idiv,
            "udiv" => Token::Udiv,
            "irem" => Token::Irem,
            "urem" => Token::Urem,
            "iabs" => Token::IAbs,
            "ineg" => Token::INeg,

            "iand" => Token::IAnd,
            "ior" => Token::IOr,
            "ixor" => Token::IXor,
            "inot" => Token::INot,
            "ishl" => Token::IShl,
            "ishr" => Token::IShr,
            "isar" => Token::ISar,

            "eq" => Token::Ieq,
            "ne" => Token::Ine,
            "slt" => Token::Islt,
            "sle" => Token::Isle,
            "sgt" => Token::Isgt,
            "sge" => Token::Isge,

            "ult" => Token::Iult,
            "ule" => Token::Iule,
            "ugt" => Token::Iugt,
            "uge" => Token::Iuge,

            "fadd" => Token::Fadd,
            "fsub" => Token::Fsub,
            "fmul" => Token::Fmul,
            "fdiv" => Token::Fdiv,
            "frem" => Token::Frem,
            "fabs" => Token::FAbs,
            "fneg" => Token::FNeg,

            "feq" => Token::Feq,
            "fne" => Token::Fne,
            "folt" => Token::Folt,
            "fole" => Token::Fole,
            "fogt" => Token::Fogt,
            "foge" => Token::Foge,
            "fult" => Token::Fult,
            "fule" => Token::Fule,
            "fugt" => Token::Fugt,
            "fuge" => Token::Fuge,

            "cast" => Token::Cast,
            "assert" => Token::Assert,

            "ret" => Token::Ret,
            "br" => Token::Br,
            "brtrue" => Token::Brtrue,
            "brfalse" => Token::Brfalse,

            "call" => Token::Call,

            "pop" => Token::Pop,
            "dup" => Token::Dup,
            "swp" => Token::Swp,
            "over" => Token::Over,
            "nip" => Token::Nip,
            "tuck" => Token::Tuck,
            "rot" => Token::Rot,
            "pick" => Token::Pick,
            "roll" => Token::Roll,

            "i1" => Token::I1,
            "i8" => Token::I8,
            "i16" => Token::I16,
            "i32" => Token::I32,
            "i64" => Token::I64,
            "f32" => Token::F32,
            "f64" => Token::F64,

            "locals" => Token::Locals,

            _ => Token::Ident(text.to_string()),
        }
    }

    fn reference(&mut self, prefix: char) -> Result<Token, LexError> {
        self.advance();

        let token = match self.peek() {
            Some(ch) if ch.is_ascii_digit() => {
                let start = self.pos;

                while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                    self.advance();
                }

                let text = &self.source[start..self.pos];

                let value = text
                    .parse::<i64>()
                    .map_err(|_| self.error(LexErrorKind::InvalidReference))?;

                Token::IntLiteral(value)
            }

            Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => self.identifier(),

            _ => {
                return Err(self.error(LexErrorKind::ExpectedReference(prefix)));
            }
        };

        let inner = Box::new(token);

        match prefix {
            '#' => Ok(Token::ArgRef(inner)),
            '%' => Ok(Token::LocalRef(inner)),
            _ => unreachable!(),
        }
    }

    fn number(&mut self) -> Result<Token, LexError> {
        let start = self.pos;

        let negative = if self.peek() == Some('-') {
            self.advance();
            true
        } else {
            false
        };

        let radix = if self.peek() == Some('0') {
            match self.peek_next() {
                Some('x' | 'X') => {
                    self.advance();
                    self.advance();
                    16
                }
                Some('b' | 'B') => {
                    self.advance();
                    self.advance();
                    2
                }
                Some('o' | 'O') => {
                    self.advance();
                    self.advance();
                    8
                }
                Some(ch) if ch.is_ascii_digit() => {
                    // Leading zero means octal: 0777.
                    self.advance();
                    8
                }
                _ => 10,
            }
        } else {
            10
        };

        if radix != 10 || (self.peek() == Some('0') && self.pos == start + usize::from(negative)) {
            let digits_start = self.pos;

            while self.peek().is_some_and(|ch| ch.is_digit(radix)) {
                self.advance();
            }

            if digits_start == self.pos {
                return Err(self.error(LexErrorKind::ExpectedIntegerDigits));
            }

            let text = &self.source[digits_start..self.pos];

            let value = i64::from_str_radix(text, radix)
               .map_err(|_| self.error(LexErrorKind::IntegerOutOfRange))?;

            let value = if negative {
                value
                    .checked_neg()
.ok_or_else(|| self.error(LexErrorKind::IntegerOutOfRange))?
            } else {
                value
            };

            return Ok(Token::IntLiteral(value));
        }

        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance();
        }

        let mut is_float = false;

        if self.peek() == Some('.') && self.peek_next().is_some_and(|ch| ch.is_ascii_digit()) {
            is_float = true;
            self.advance();

            while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance();
            }
        }

        if matches!(self.peek(), Some('e' | 'E')) {
            is_float = true;
            self.advance();

            if matches!(self.peek(), Some('+' | '-')) {
                self.advance();
            }

            let exponent_start = self.pos;

            while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance();
            }

            if exponent_start == self.pos {
               return Err(self.error(LexErrorKind::ExpectedExponentDigits));
            }
        }

        let text = &self.source[start..self.pos];

        if is_float {
            text.parse::<f64>()
               .map_err(|_| self.error(LexErrorKind::InvalidFloat))?;

            Ok(Token::FloatLiteral(text.to_string()))
        } else {
            text.parse::<i64>()
                .map(Token::IntLiteral)
                .map_err(|_| self.error(LexErrorKind::IntegerOutOfRange))
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            let Some(ch) = self.peek() else {
                break;
            };

            if ch == ';' && self.peek_next() == Some(';') {
                self.skip_comment();
                continue;
            }

            if ch == '/' && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                self.skip_comment();
                continue;
            }

            let token = match ch {
                '{' => {
                    self.advance();
                    Token::LBrace
                }
                '}' => {
                    self.advance();
                    Token::RBrace
                }
                '(' => {
                    self.advance();
                    Token::LParen
                }
                ')' => {
                    self.advance();
                    Token::RParen
                }
                '[' => {
                    self.advance();
                    Token::LBracket
                }
                ']' => {
                    self.advance();
                    Token::RBracket
                }
                ':' => {
                    self.advance();
                    Token::Colon
                }
                ',' => {
                    self.advance();
                    Token::Comma
                }
                '.' => {
                    self.advance();

                    let start = self.pos;

                    while self
                        .peek()
                        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    {
                        self.advance();
                    }

                    Token::Ident(format!(".{}", &self.source[start..self.pos]))
                }
                '#' | '%' => self.reference(ch)?,
                '-' | '0'..='9' => self.number()?,
                c if c.is_ascii_alphabetic() || c == '_' => self.identifier(),
                _ => {
                    return Err(self.error(LexErrorKind::UnexpectedCharacter(ch)));
                }
            };

            tokens.push(token);
        }

        Ok(tokens)
    }
}
