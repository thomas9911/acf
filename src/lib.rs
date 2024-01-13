use std::iter::{Enumerate, Peekable};
use std::ops::Range;
use std::str::Chars;

use winnow::ascii::alphanumeric1;
use winnow::combinator::{delimited, separated, separated_pair};
use winnow::error::{ErrMode, InputError, ParseError};
use winnow::prelude::*;
use winnow::token::take_while;

const SPECIAL_CHARS: [char; 5] = ['=', ',', '{', '}', ':'];

fn xd((index, ch): &(usize, char)) -> bool {
    SPECIAL_CHARS.contains(ch)
}

// #[derive(Debug)]
pub struct Tokenizer<'a> {
    data: &'a str,
    // chars: Peekable<Enumerate<Chars<'a>>>,
    chars: Box<dyn Iterator<Item = (usize, char)> + 'a>,
    last_char: Option<(usize, char)>,
    start: usize,
    stop: bool,
    next_range: Option<Range<usize>>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(data: &'a str) -> Self {
        Tokenizer {
            data,
            chars: Box::new(data.chars().enumerate()),
            last_char: None,
            start: 0,
            stop: false,
            next_range: Some(0..1),
        }
    }
}

// impl<'a> Iterator for Tokenizer<'a> {
//     type Item = Token;

//     fn next(&mut self) -> Option<Self::Item> {
//         // let range = if let Some(current_range) = self.next_range.take() {
//         //     current_range
//         // } else if let Some((current_postion, ch)) = self.chars.next() {
//         //     let range = self.start..current_postion;
//         //     self.next_range = Some(current_postion..(current_postion+1));
//         //     self.start = (current_postion+1);
//         //     range
//         // } else {
//         //     if self.stop {
//         //         return None;
//         //     } else {
//         //         self.stop = true;
//         //         self.start..self.data.len()
//         //     }
//         // };
//         // let range = if let Some(range) = self.next_range.take() {
//         //     Some(range)
//         // } else {
//         //     let mut range = None;
//         //     while let Some((index, ch)) = self.chars.next() {
//         //         if SPECIAL_CHARS.contains(&ch) {
//         //             if let Some((prev_index, prev_ch)) = self.last_char {
//         //                 self.next_range = Some(prev_index+1..index);
//         //                 range = Some(self.start..prev_index+1);
//         //                 self.last_char = None;
//         //                 self.start = prev_index;
//         //                 break;
//         //             }
//         //             // range = Some(self.start..index);
//         //             // self.start = index;
//         //             // break;
//         //             self.last_char = Some((index, ch));
//         //             continue;
//         //         }

//         //     }
//         //     range
//         // };

//         // dbg!(&range);

//         // let range = range?;
//         // if range.len() > 1 {
//         //     return Some(Token {
//         //         kind: TokenKind::String,
//         //         range,
//         //     });
//         // }
//         // let kind = match &self.data[range.clone()] {
//         //     "=" => TokenKind::Assignment,
//         //     "," => TokenKind::ItemSeperator,
//         //     "{" => TokenKind::BracketOpen,
//         //     "}" => TokenKind::BracketClose,
//         //     ":" => TokenKind::KeySeperator,
//         //     e => panic!("character left: '{}'", e),
//         // };

//         // Some(Token { kind, range })

//         if self.stop {
//             return None;
//         }

//         let mut start = self.start;
//         let mut end = self.data.len();

//         while let Some((index, ch)) = self.chars.next() {
//             end = index;

//             if !SPECIAL_CHARS.contains(&ch) {
//                 continue;
//             }

//             if start != end {
//                 self.start = end + 1;
//                 break;
//             }
//         }

//         if end == self.data.len() {
//             self.stop = true;
//         }

//         let range = start..end;
//         if range.len() > 1 {
//             return Some(Token {
//                 kind: TokenKind::String,
//                 range,
//             });
//         }
//         let kind = match &self.data[range.clone()] {
//             "=" => TokenKind::Assignment,
//             "," => TokenKind::ItemSeperator,
//             "{" => TokenKind::BracketOpen,
//             "}" => TokenKind::BracketClose,
//             ":" => TokenKind::KeySeperator,
//             "" => return None,
//             e => panic!("character left: '{}'", e),
//         };

//         Some(Token { kind, range })
//     }
// }

fn value_parser<'s>(input: &mut &'s str) -> PResult<&'s str, InputError<&'s str>> {
    // alphanumeric1.parse_next(input)
    take_while(0.., |ch| !SPECIAL_CHARS.contains(&ch)).parse_next(input)
}

fn parser4<'s>(input: &mut &'s str) -> PResult<(&'s str, &'s str), InputError<&'s str>> {
    separated_pair(value_parser, (ws, ":", ws), value_parser).parse_next(input)
}

fn parser3<'s>(input: &mut &'s str) -> PResult<Vec<(&'s str, &'s str)>, InputError<&'s str>> {
    separated(0.., parser4, (ws, ",", ws)).parse_next(input)
}

fn parser2<'s>(input: &mut &'s str) -> PResult<Vec<(&'s str, &'s str)>, InputError<&'s str>> {
    delimited(("{", ws), parser3, (ws, "}", ws)).parse_next(input)
}

fn parser1<'s>(
    input: &mut &'s str,
) -> PResult<(&'s str, Vec<(&'s str, &'s str)>), InputError<&'s str>> {
    separated_pair(value_parser, (ws, "=", ws), parser2).parse_next(input)
}

// fn parser0<'s>(input: &mut &'s str) -> Result<Vec<(&'s str, Vec<(&'s str, &'s str)>)>, ParseError<&'s str, InputError<&'s str>>> {
fn parser0<'s>(
    input: &mut &'s str,
) -> PResult<Vec<(&'s str, Vec<(&'s str, &'s str)>)>, InputError<&'s str>> {
    separated(0.., parser1, (ws, ",", ws)).parse_next(input)
}

fn ws<'s>(input: &mut &'s str) -> PResult<&'s str, InputError<&'s str>> {
    // Combinators like `take_while` return a function. That function is the
    // parser,to which we can pass the input
    take_while(0.., WS).parse_next(input)
}

const WS: &[char] = &[' ', '\t', '\r', '\n'];

pub fn parse_flat<'s>(
    data: &mut &'s str,
) -> Result<Vec<(&'s str, Vec<(&'s str, &'s str)>)>, ParseError<&'s str, InputError<&'s str>>> {
    parser0.parse(data)
}

// fn string_data<'a>(data: &'a mut &str) -> PResult<Token> {
//     take_while(0.., |ch| !SPECIAL_CHARS.contains(&ch)).map(|input| Token{kind: TokenKind::String, range: 0..1}).parse_next(data)
// }

#[derive(Debug)]
pub struct Token {
    kind: TokenKind,
    range: Range<usize>,
}

#[derive(Debug)]
pub enum TokenKind {
    String,
    Assignment,
    BracketOpen,
    BracketClose,
    KeySeperator,
    ItemSeperator,
}

#[test]
fn tokenize_this() {
    let mut data = r#"config1={value: 1, default: 12, yes: true},config2={DEFAULT: "testing"}"#;
    // let mut data = r#"config1={value: 1, default: 12}"#;
    let out = parse_flat(&mut data).unwrap();
    let expected = vec![
        (
            "config1",
            vec![("value", "1"), ("default", "12"), ("yes", "true")],
        ),
        ("config2", vec![("DEFAULT", "\"testing\"")]),
    ];

    assert_eq!(out, expected);
}
