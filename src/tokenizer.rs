use kstring::KString;
use logos::{Logos, Span};
use std::{borrow::Cow, fmt::Display, num::ParseFloatError};
use std::{iter::Peekable, num::ParseIntError};

use crate::{StringMap, ACF};

// const SPECIAL_CHARS: [char; 5] = ['=', ',', '{', '}', ':'];

pub type Lexer<'a> = logos::Lexer<'a, Token<'a>>;
pub type PeekableLexer<'a> = Peekable<Lexer<'a>>;

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
        match self {
            TokenizeErrorKind::ParseInt(parse_int_error) => parse_int_error.fmt(f),
            TokenizeErrorKind::ParseFloat(parse_float_error) => parse_float_error.fmt(f),
            TokenizeErrorKind::Unescape(unescape_error) => unescape_error.fmt(f),
            TokenizeErrorKind::Other => f.write_str("other"),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct TokenizeError {
    span: Span,
    error: TokenizeErrorKind,
}

impl Display for TokenizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: include the slice here as well?
        write!(
            f,
            "tokenize error: {} on {}..{}",
            self.error, self.span.start, self.span.end
        )
    }
}

impl TokenizeError {
    fn from_lexer<'a>(lexer: &mut Lexer<'a>) -> Self {
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

fn to_boolean<'a>(lexer: &mut Lexer<'a>) -> bool {
    match lexer.slice() {
        "true" => true,
        "false" => false,
        _ => unreachable!(),
    }
}

fn to_integer<'a>(lexer: &mut Lexer<'a>) -> Result<i64, TokenizeError> {
    lexer
        .slice()
        .replace("_", "")
        .parse()
        .map_err(|e| TokenizeError::from_parse_int_error(e, lexer.span()))
}

fn to_float<'a>(lexer: &mut Lexer<'a>) -> Result<f64, TokenizeError> {
    lexer
        .slice()
        .parse()
        .map_err(|e| TokenizeError::from_parse_float_error(e, lexer.span()))
}

fn to_unquote_string<'a>(lexer: &mut Lexer<'a>) -> Result<Cow<'a, str>, TokenizeError> {
    if lexer.slice().len() >= 2 {
        let unescaped = snailquote::unescape(lexer.slice())
            .map_err(|e| TokenizeError::from_unescaped_error(e, lexer.span()))?;
        Ok(Cow::Owned(unescaped))
    } else {
        panic!("unquote regex is invalid")
    }
}

fn slice_to_cow<'a>(lexer: &mut Lexer<'a>) -> Cow<'a, str> {
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

pub fn tokenize<'a>(input: &'a str) -> Lexer<'a> {
    Token::lexer(input)
}

fn parse_expect<'a>(
    lexer: &mut PeekableLexer<'a>,
    expected: TokenKind,
) -> Result<Token<'a>, String> {
    match lexer.next() {
        None => Err(format!("unexpected end, expected {:?}", expected)),
        Some(Ok(token)) if !expected.matches(&token) => Err(format!(
            "unexpected token, expected {:?} but found {:?}",
            expected,
            token.kind()
        )),
        Some(Err(e)) => Err(e.to_string()),
        Some(Ok(token)) => Ok(token),
    }
}

pub fn parse<'a>(lexer: Lexer<'a>) -> Result<ACF, String> {
    let mut lexer = lexer.peekable();
    parse_root(&mut lexer)
}

fn parse_root<'a>(lexer: &mut PeekableLexer<'a>) -> Result<ACF, String> {
    let mut entity = StringMap::default();
    while let Some(_) = lexer.peek() {
        // let key = parse_expect(lexer, TokenKind::String)?;
        // parse_expect(lexer, TokenKind::Equal)?;
        // let value = parse_value(lexer)?;
        let (key, value) = parse_map_item(lexer, TokenKind::Equal)?;

        entity.insert(key, value);

        if parse_end_of_sequence(lexer, None)? {
            break;
        }
    }

    Ok(ACF::Map(entity))
}

fn parse_value<'a>(lexer: &mut PeekableLexer<'a>) -> Result<ACF, String> {
    let current_token = lexer
        .next()
        .ok_or_else(|| String::from("unexpected end, expected value"))?
        .map_err(|e| e.to_string())?;

    let value = match current_token {
        Token::Bool(bool) => ACF::Boolean(bool),
        Token::Float(float) => ACF::Float(float.into()),
        Token::Integer(integer) => ACF::Integer(integer),
        Token::String(string) => match string {
            Cow::Borrowed(str) => ACF::String(KString::from_ref(&str)),
            Cow::Owned(string) => ACF::String(KString::from_string(string)),
        },
        Token::BracketOpen => parse_map_or_seq(lexer)?,
        _ => {
            return Err(format!(
                "unexpected token, expected {:?} but found {:?}",
                TokenKind::Value,
                current_token.kind()
            ))
        }
    };

    Ok(value)
}

fn parse_map_or_seq<'a>(lexer: &mut PeekableLexer<'a>) -> Result<ACF, String> {
    // is already missing the first {

    // you cannot mix map and seq in one entity
    let is_map;
    let mut entity;
    match parse_map_or_seq_item(lexer, TokenKind::Colon)? {
        Item::Seq(value) => {
            is_map = false;
            entity = ACF::Seq(vec![value]);
            if parse_end_of_sequence(lexer, Some(TokenKind::BracketClose))? {
                // just one item
                parse_expect(lexer, TokenKind::BracketClose)?;
                return Ok(entity);
            };
        }
        Item::Map(key, value) => {
            is_map = true;
            entity = ACF::Map(Default::default());
            let map_ref = entity.as_map_mut().expect("we just set this to map");
            map_ref.insert(key, value);
            if parse_end_of_sequence(lexer, Some(TokenKind::BracketClose))? {
                // just one item
                parse_expect(lexer, TokenKind::BracketClose)?;
                return Ok(entity);
            };
        }
    }

    while let Some(token) = lexer.peek() {
        dbg!(&token);
        if token.as_ref().map(|x| x.kind()) == Ok(TokenKind::BracketClose) {
            lexer.next();
            break;
        }
        if is_map {
            let (key, value) = parse_map_item(lexer, TokenKind::Colon)?;
            let map_ref = entity.as_map_mut().expect("we just set this to map");
            map_ref.insert(key, value);
        } else {
            let value = parse_value(lexer)?;
            let seq_ref = entity.as_seq_mut().expect("we just set this to seq");
            seq_ref.push(value);
        }

        if parse_end_of_sequence(lexer, Some(TokenKind::BracketClose))? {
            break;
        };
    }

    parse_expect(lexer, TokenKind::BracketClose)?;

    Ok(entity)
}

fn parse_end_of_sequence<'a>(
    lexer: &mut PeekableLexer<'a>,
    end_token: Option<TokenKind>,
) -> Result<bool, String> {
    // comma
    let peeked = lexer.peek();
    let peeked_kind = peeked.as_ref().map(|x| x.as_ref().map(|y| y.kind()));
    if end_token.is_none() && peeked.is_none() {
        return Ok(true);
    }
    if end_token.is_some() && peeked_kind == Some(Ok(end_token.expect("checked is some"))) {
        // no trailing comma
        return Ok(true);
    }
    if peeked_kind == Some(Ok(TokenKind::Comma)) {
        parse_expect(lexer, TokenKind::Comma)?;
    }
    Ok(false)
}

fn parse_map_item<'a>(
    lexer: &mut PeekableLexer<'a>,
    separator: TokenKind,
) -> Result<(String, ACF), String> {
    let key = parse_expect(lexer, TokenKind::String)?;
    let Token::String(key) = key else {
        unreachable!("parse_expect bug, expected string")
    };
    parse_expect(lexer, separator)?;
    let value = parse_value(lexer)?;
    dbg!(&value);

    Ok((key.to_string(), value))
}

enum Item {
    Map(String, ACF),
    Seq(ACF),
}

fn parse_map_or_seq_item<'a>(
    lexer: &mut PeekableLexer<'a>,
    separator: TokenKind,
) -> Result<Item, String> {
    let value = parse_value(lexer)?;
    match lexer.peek() {
        Some(Ok(s)) if s.kind() == separator => {
            let key = value
                .into_string()
                .ok_or_else(|| format!("unexpected token, expected String"))?;
            lexer.next();
            let value = parse_value(lexer)?;

            Ok(Item::Map(key, value))
        }
        _ => return Ok(Item::Seq(value)),
    }
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

// #[test]
// fn tokenize_this2() {
//     // let data = r#"config1={value: 1, default: 12, yes: true}"#;
//     // let data = r#"config1={a: "extra \"quote\""}"#;
//     // let data = r#"config1={value: 1, default: 1_2, yes: true, number: 1.23},config2={DEFAULT: "testing"}"#;
//     let data = r#"config={a: 1, b: 1.015, c: 3.1415, d: 10009, e: 0, f: -19.34, g: -2.17e-14, h: 1_2},cheese=1"#;
//     let mut out = tokenize(data).spanned();

//     while let Some((result, range)) = out.next() {
//         println!("{:?}, {:?}", range, result.unwrap());
//     }
//     // let mut strings = Vec::new();
//     // debug_visit_ast(data, &out, &mut strings);
//     // let expected = vec![('m', "config1=testing"), ('k', "config1"), ('s', "testing")];

//     // assert_eq!(expected, strings);
//     panic!()
// }

#[cfg(test)]
use crate::{acf_map, acf_seq};

#[test]
fn parse_test_map_with_numbers() {
    let data = r#"config={a: 1, b: 1.015, c: 3.1415, d: 10009, e: 0, f: -19.34, g: -2.17e-14, h: 1_2},cheese=1"#;
    let expected = acf_map! {
        "config" => acf_map! {
            "a" => 1,
            "b" => 1.015,
            "c" => 3.1415,
            "d" => 10009,
            "e" => 0,
            "f" => -19.34,
            "g" => -2.17e-14,
            "h" => 12,
        },
        "cheese" => 1
    };

    let lexer = tokenize(data);

    assert_eq!(expected, parse(lexer).unwrap());
}

#[test]
fn parse_test_seq_with_numbers() {
    let data = r#"config={1, 1.015, 3.1415, 10009, 0, -19.34, -2.17e-14, 1_2},cheese=1,"#;
    let expected = acf_map! {
        "config" => acf_seq! {
            1,
            1.015,
            3.1415,
            10009,
            0,
            -19.34,
            -2.17e-14,
            12,
        },
        "cheese" => 1
    };

    let lexer = tokenize(data);

    assert_eq!(expected, parse(lexer).unwrap());
}
