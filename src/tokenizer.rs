use logos::{Lexer, Logos, Source, Span};
use std::num::ParseIntError;
use std::{borrow::Cow, fmt::Display, num::ParseFloatError};

use crate::ACF;

const SPECIAL_CHARS: [char; 5] = ['=', ',', '{', '}', ':'];

#[derive(Default, Debug, Clone, PartialEq)]
pub enum TokenizeErrorKind {
    ParseInt(ParseIntError),
    ParseFloat(ParseFloatError),
    Unescape(snailquote::UnescapeError),
    #[default]
    Other,
}

impl Display for TokenizeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct TokenizeError {
    span: Span,
    error: TokenizeErrorKind,
}

impl Display for TokenizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

// impl From<ParseIntError> for TokenizeError  {
//     fn from(value: ParseIntError) -> Self {
//         TokenizeError::ParseInt(value)
//     }
// }
impl TokenizeError {
    fn from_lexer<'a>(lexer: &mut logos::Lexer<'a, Token<'a>>) -> Self {
        TokenizeError {
            span: lexer.span(),
            error: TokenizeErrorKind::default(),
        }
    }

    fn from_parse_int_error(err: ParseIntError, span: Span) -> Self {
        TokenizeError {
            span,
            error: TokenizeErrorKind::ParseInt(err),
        }
    }

    fn from_parse_float_error(err: ParseFloatError, span: Span) -> Self {
        TokenizeError {
            span,
            error: TokenizeErrorKind::ParseFloat(err),
        }
    }

    fn from_unescaped_error(err: snailquote::UnescapeError, span: Span) -> Self {
        TokenizeError {
            span,
            error: TokenizeErrorKind::Unescape(err),
        }
    }
}

fn to_boolean<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> bool {
    match lexer.slice() {
        "true" => true,
        "false" => false,
        _ => unreachable!(),
    }
}

fn to_integer<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Result<i64, TokenizeError> {
    lexer
        .slice()
        .replace("_", "")
        .parse()
        .map_err(|e| TokenizeError::from_parse_int_error(e, lexer.span()))
}

fn to_float<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Result<f64, TokenizeError> {
    lexer
        .slice()
        .parse()
        .map_err(|e| TokenizeError::from_parse_float_error(e, lexer.span()))
}

fn to_unquote_string<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Result<Cow<'a, str>, TokenizeError> {
    if lexer.slice().len() >= 2 {
        let unescaped = snailquote::unescape(lexer.slice())
            .map_err(|e| TokenizeError::from_unescaped_error(e, lexer.span()))?;
        Ok(Cow::Owned(unescaped))
    } else {
        panic!("unquote regex is invalid")
    }
}

fn slice_to_cow<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Cow<'a, str> {
    lexer.slice().into()
}

#[derive(Logos, Debug, PartialEq)]
#[logos(error(TokenizeError, TokenizeError::from_lexer))]
#[logos(skip r"\s+")]
#[logos(subpattern leading_integer = r"([[:digit:]]+)")]
#[logos(subpattern integer = r"([0-9]|[1-9](_?[0-9])+)")]
#[logos(subpattern base_integer = r"[-+]?(?&integer)")]
#[logos(subpattern simple_float = r"(?&base_integer)\.(?&leading_integer)")]
#[logos(subpattern exp_float = r"(?&simple_float)e(?&base_integer)")]
pub enum Token<'a> {
    #[regex("true|false", to_boolean)]
    Bool(bool),
    #[regex("(?&base_integer)", to_integer, priority = 200)]
    Integer(i64),
    #[regex(r"(?&simple_float)|(?&exp_float)", to_float)]
    Float(f64),
    #[regex(r#""(?:\\.|[^"\\])*""#, to_unquote_string, priority = 10)]
    #[regex("[^=,{}:\\s]+", slice_to_cow)]
    String(Cow<'a, str>),
    #[token("=")]
    Equal,
    #[token(",")]
    Comma,
    #[token("{")]
    BracketOpen,
    #[token("}")]
    BracketClose,
    #[token(":")]
    Colon,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TokenKind {
    Bool,
    Integer,
    Float,
    String,
    Equal,
    Comma,
    BracketOpen,
    BracketClose,
    Colon,
    /// group of all value kinds
    Value,
}

impl<'a> Token<'a> {
    pub fn kind(&self) -> TokenKind {
        match self {
            Token::Bool(_) => TokenKind::Bool,
            Token::Integer(_) => TokenKind::Integer,
            Token::Float(_) => TokenKind::Float,
            Token::String(_) => TokenKind::String,
            Token::Equal => TokenKind::Equal,
            Token::Comma => TokenKind::Comma,
            Token::BracketOpen => TokenKind::BracketOpen,
            Token::BracketClose => TokenKind::BracketClose,
            Token::Colon => TokenKind::Colon,
        }
    }
}

impl TokenKind {
    pub fn matches(&self, token: &Token<'_>) -> bool {
        if self == &token.kind() {
            return true;
        }
        if self == &TokenKind::Value
            && [
                TokenKind::Bool,
                TokenKind::Float,
                TokenKind::Integer,
                TokenKind::String,
            ]
            .contains(&token.kind())
        {
            return true;
        }

        return false;
    }
}

// pub fn tokenize<'a>(input: &'a str) -> impl Iterator<Item = Result<Token<'a>, TokenizeError>> + 'a {
//     Token::lexer(input)
// }
pub fn tokenize<'a>(input: &'a str) -> Lexer<'a, Token<'a>> {
    Token::lexer(input)
}

fn parse_expect<'a>(
    lexer: &mut Lexer<'a, Token<'a>>,
    expected: TokenKind,
) -> Result<Token<'a>, String> {
    match lexer.next() {
        None => Err(format!("unexpected end, expected {:?}", expected)),
        Some(Ok(token)) if expected.matches(&token) => Err(format!(
            "unexpected token, expected {:?} but found {:?}",
            expected,
            token.kind()
        )),
        Some(Err(e)) => Err(e.to_string()),
        Some(Ok(token)) => Ok(token),
    }
}

pub fn parse<'a>(mut lexer: Lexer<'a, Token<'a>>) -> Result<ACF, String> {
    parse_root(&mut lexer)
}

fn parse_root<'a>(lexer: &mut Lexer<'a, Token<'a>>) -> Result<ACF, String> {
    let key = parse_expect(lexer, TokenKind::String)?;
    parse_expect(lexer, TokenKind::Equal)?;
    // TODO: doesnt work because of array and map
    let value = parse_expect(lexer, TokenKind::Value)?;

    Ok(ACF::Boolean(true))
}

#[test]
fn tokenize_with_numbers() {
    let data = r#"config={a: 1, b: 1.015, c: 3.1415, d: 10009, e: 0, f: -19.34, g: -2.17e-14, h: 1_2},cheese=1"#;
    let out: Result<Vec<_>, _> = tokenize(data).collect();

    let expected = vec![
        Token::String("config".into()),
        Token::Equal,
        Token::BracketOpen,
        Token::String("a".into()),
        Token::Colon,
        Token::Integer(1),
        Token::Comma,
        Token::String("b".into()),
        Token::Colon,
        Token::Float(1.015),
        Token::Comma,
        Token::String("c".into()),
        Token::Colon,
        Token::Float(3.1415),
        Token::Comma,
        Token::String("d".into()),
        Token::Colon,
        Token::Integer(10009),
        Token::Comma,
        Token::String("e".into()),
        Token::Colon,
        Token::Integer(0),
        Token::Comma,
        Token::String("f".into()),
        Token::Colon,
        Token::Float(-19.34),
        Token::Comma,
        Token::String("g".into()),
        Token::Colon,
        Token::Float(-2.17e-14),
        Token::Comma,
        Token::String("h".into()),
        Token::Colon,
        Token::Integer(12),
        Token::BracketClose,
        Token::Comma,
        Token::String("cheese".into()),
        Token::Equal,
        Token::Integer(1),
    ];

    assert_eq!(expected, out.unwrap());
}

#[test]
fn tokenize_this2() {
    // let data = r#"config1={value: 1, default: 12, yes: true}"#;
    // let data = r#"config1={a: "extra \"quote\""}"#;
    // let data = r#"config1={value: 1, default: 1_2, yes: true, number: 1.23},config2={DEFAULT: "testing"}"#;
    let data = r#"config={a: 1, b: 1.015, c: 3.1415, d: 10009, e: 0, f: -19.34, g: -2.17e-14, h: 1_2},cheese=1"#;
    let mut out = tokenize(data).spanned();

    while let Some((result, range)) = out.next() {
        println!("{:?}, {:?}", range, result.unwrap());
    }
    // let mut strings = Vec::new();
    // debug_visit_ast(data, &out, &mut strings);
    // let expected = vec![('m', "config1=testing"), ('k', "config1"), ('s', "testing")];

    // assert_eq!(expected, strings);
    panic!()
}

#[test]
fn parse_test() {
    let data = r#"config={a: 1, b: 1.015, c: 3.1415, d: 10009, e: 0, f: -19.34, g: -2.17e-14, h: 1_2},cheese=1"#;
    let lexer = tokenize(data);

    dbg!(parse(lexer).unwrap());
    panic!()
}
