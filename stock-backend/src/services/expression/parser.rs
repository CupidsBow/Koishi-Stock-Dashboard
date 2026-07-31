//! Recursive-descent expression parser for Alpha factor formulas.
//!
//! Currently validates expression syntax and extracts metadata (column references,
//! function calls) but does not compile to executable code. Factor evaluation
//! uses the hard-coded functions in `compute.rs` instead.
//!
//! Grammar:
//! ```text
//! Expr    ::= Term (('+' | '-') Term)*
//! Term    ::= Factor (('*' | '/') Factor)*
//! Factor  ::= Number | FuncCall | '(' Expr ')' | ColumnRef
//! FuncCall ::= ident '(' args (',' args)* ')'
//! ColumnRef ::= 'Open' | 'High' | 'Low' | 'Close' | 'Volume'
//! Number  ::= '-'? [0-9]+ ('.' [0-9]+)?
//! ```

use anyhow::{bail, Result};

// ── AST ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ExprNode {
    Number(f64),
    ColumnRef(String),
    FuncCall {
        name: String,
        args: Vec<ExprNode>,
    },
    Add(Box<ExprNode>, Box<ExprNode>),
    Sub(Box<ExprNode>, Box<ExprNode>),
    Mul(Box<ExprNode>, Box<ExprNode>),
    Div(Box<ExprNode>, Box<ExprNode>),
}

// ── Tokenizer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Number(f64),
    LParen,
    RParen,
    Plus,
    Minus,
    Star,
    Slash,
    Comma,
}

struct Tokenizer {
    chars: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().filter(|c| !c.is_whitespace()).collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        while self.pos < self.chars.len() {
            let c = self.peek().unwrap();
            match c {
                '(' => {
                    self.advance();
                    tokens.push(Token::LParen);
                }
                ')' => {
                    self.advance();
                    tokens.push(Token::RParen);
                }
                '+' => {
                    self.advance();
                    tokens.push(Token::Plus);
                }
                '-' => {
                    self.advance();
                    // Only treat '-' as a negative number prefix when it starts
                    // a new expression: after '(', ',', another operator, or at
                    // the very beginning.  Otherwise it is subtraction.
                    let is_neg_context = tokens.is_empty()
                        || matches!(
                            tokens.last(),
                            Some(Token::LParen)
                                | Some(Token::Comma)
                                | Some(Token::Plus)
                                | Some(Token::Minus)
                                | Some(Token::Star)
                                | Some(Token::Slash)
                        );
                    if is_neg_context {
                        if let Some(&next) = self.chars.get(self.pos) {
                            if next.is_ascii_digit() || next == '.' {
                                tokens.push(self.read_number(true)?);
                                continue;
                            }
                        }
                    }
                    tokens.push(Token::Minus);
                }
                '*' => {
                    self.advance();
                    tokens.push(Token::Star);
                }
                '/' => {
                    self.advance();
                    tokens.push(Token::Slash);
                }
                ',' => {
                    self.advance();
                    tokens.push(Token::Comma);
                }
                c if c.is_ascii_digit() || c == '.' => {
                    tokens.push(self.read_number(false)?);
                }
                c if c.is_alphabetic() || c == '_' => {
                    tokens.push(self.read_ident());
                }
                _ => bail!("unexpected character '{}' at position {}", c, self.pos),
            }
        }
        Ok(tokens)
    }

    fn read_number(&mut self, neg: bool) -> Result<Token> {
        let start = self.pos;
        let mut saw_dot = false;
        while let Some(&c) = self.chars.get(self.pos) {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else if c == '.' && !saw_dot {
                saw_dot = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        let mut v: f64 = s
            .parse()
            .map_err(|e| anyhow::anyhow!("bad number '{}': {}", s, e))?;
        if neg {
            v = -v;
        }
        Ok(Token::Number(v))
    }

    fn read_ident(&mut self) -> Token {
        let start = self.pos;
        while let Some(&c) = self.chars.get(self.pos) {
            if c.is_alphanumeric() || c == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let s: String = self.chars[start..self.pos].iter().collect();
        Token::Ident(s)
    }
}

// ── Parser ───────────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    /// Expr ::= Term (('+' | '-') Term)*
    fn parse_expr(&mut self) -> Result<ExprNode> {
        let mut left = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.advance();
                    left = ExprNode::Add(Box::new(left), Box::new(self.parse_term()?));
                }
                Token::Minus => {
                    self.advance();
                    left = ExprNode::Sub(Box::new(left), Box::new(self.parse_term()?));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// Term ::= Factor (('*' | '/') Factor)*
    fn parse_term(&mut self) -> Result<ExprNode> {
        let mut left = self.parse_factor()?;
        while let Some(tok) = self.peek() {
            match tok {
                Token::Star => {
                    self.advance();
                    left = ExprNode::Mul(Box::new(left), Box::new(self.parse_factor()?));
                }
                Token::Slash => {
                    self.advance();
                    left = ExprNode::Div(Box::new(left), Box::new(self.parse_factor()?));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    /// Factor ::= Number | FuncCall | '(' Expr ')' | ColumnRef
    fn parse_factor(&mut self) -> Result<ExprNode> {
        match self.peek().cloned() {
            Some(Token::Number(v)) => {
                self.advance();
                Ok(ExprNode::Number(v))
            }
            Some(Token::LParen) => {
                self.advance();
                let e = self.parse_expr()?;
                match self.advance() {
                    Some(Token::RParen) => Ok(e),
                    _ => bail!("expected ')'"),
                }
            }
            Some(Token::Ident(_)) => {
                let ident = if let Token::Ident(s) = self.advance().unwrap() {
                    s.clone()
                } else {
                    unreachable!()
                };
                if self.peek() == Some(&Token::LParen) {
                    self.advance(); // '('
                    let mut args = Vec::new();
                    if self.peek() != Some(&Token::RParen) {
                        args.push(self.parse_expr()?);
                        while self.peek() == Some(&Token::Comma) {
                            self.advance();
                            args.push(self.parse_expr()?);
                        }
                    }
                    if self.advance() != Some(&Token::RParen) {
                        bail!("expected ')' after function arguments");
                    }
                    Ok(ExprNode::FuncCall { name: ident, args })
                } else {
                    validate_column(&ident)?;
                    Ok(ExprNode::ColumnRef(ident.to_lowercase()))
                }
            }
            _ => bail!("unexpected token: {:?}", self.peek()),
        }
    }
}

// ── Validation ───────────────────────────────────────────────────────────────

fn validate_column(raw: &str) -> Result<()> {
    match raw {
        "Open" | "open" | "High" | "high" | "Low" | "low" | "Close" | "close" | "Volume"
        | "volume" => Ok(()),
        other => bail!(
            "unknown column '{}' — expected Open/High/Low/Close/Volume",
            other
        ),
    }
}

/// Known function names that are valid in factor expressions.
const KNOWN_FUNCTIONS: &[&str] = &[
    "Ts_Mean", "Ts_Std", "Ts_Max", "Ts_Min", "Ts_Corr", "Ts_Rank", "Delta", "Delay", "Rank",
    "Scale", "Log", "Abs", "Sign",
];

fn validate_function(name: &str) -> Result<()> {
    if KNOWN_FUNCTIONS.contains(&name) {
        Ok(())
    } else {
        bail!(
            "unknown function '{}' — known functions: {:?}",
            name,
            KNOWN_FUNCTIONS
        )
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Parse and validate a factor expression string.
///
/// Returns `Ok(ExprNode)` if the expression is syntactically valid and all
/// column/function references are recognized. Returns `Err(...)` otherwise.
pub fn parse_expression(input: &str) -> Result<ExprNode> {
    let mut tokenizer = Tokenizer::new(input);
    let tokens = tokenizer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expr()?;
    if parser.peek().is_some() {
        bail!(
            "trailing tokens after parsed expression: {:?}",
            parser.peek()
        );
    }
    validate_tree(&expr)?;
    Ok(expr)
}

/// Recursively validate that all column refs and function names are valid.
fn validate_tree(node: &ExprNode) -> Result<()> {
    match node {
        ExprNode::FuncCall { name, args } => {
            validate_function(name)?;
            for arg in args {
                validate_tree(arg)?;
            }
        }
        ExprNode::Add(l, r) | ExprNode::Sub(l, r) | ExprNode::Mul(l, r) | ExprNode::Div(l, r) => {
            validate_tree(l)?;
            validate_tree(r)?;
        }
        _ => {}
    }
    Ok(())
}

// ── Legacy compatibility ─────────────────────────────────────────────────────

/// Provided for backward compatibility with callers that expect a registry approach.
/// Returns an empty `HashMap` — the registry concept is not used in this simpler
/// parser; factor computation goes through `compute.rs` directly.
pub fn default_registry() -> std::collections::HashMap<String, ()> {
    std::collections::HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_column() {
        let e = parse_expression("Close").unwrap();
        assert_eq!(e, ExprNode::ColumnRef("close".into()));
    }

    #[test]
    fn test_ts_mean() {
        let e = parse_expression("Ts_Mean(Close, 5)").unwrap();
        match e {
            ExprNode::FuncCall { name, args } => {
                assert_eq!(name, "Ts_Mean");
                assert_eq!(args[0], ExprNode::ColumnRef("close".into()));
                assert_eq!(args[1], ExprNode::Number(5.0));
            }
            _ => panic!("expected FuncCall"),
        }
    }

    #[test]
    fn test_ret_formula() {
        let e = parse_expression("Delta(Close,5)/Delay(Close,5)").unwrap();
        assert!(matches!(e, ExprNode::Div(..)));
    }

    #[test]
    fn test_complex() {
        let e =
            parse_expression("(Close-Ts_Mean(Close,20))/Ts_Mean(Close,20)").unwrap();
        // (Close - Ts_Mean(Close,20)) / Ts_Mean(Close,20)
        assert!(matches!(e, ExprNode::Div(..)));
    }

    #[test]
    fn test_unknown_function() {
        let r = parse_expression("FooBar(Close, 5)");
        assert!(r.is_err());
    }

    #[test]
    fn test_unknown_column() {
        let r = parse_expression("Hello + 1");
        assert!(r.is_err());
    }

    #[test]
    fn test_all_builtin_expressions_parse() {
        use crate::services::expression::registry::builtin_factors;
        for f in builtin_factors() {
            let result = parse_expression(&f.expression);
            assert!(
                result.is_ok(),
                "failed to parse '{}' ({}): {:?}",
                f.name,
                f.expression,
                result.err()
            );
        }
    }
}