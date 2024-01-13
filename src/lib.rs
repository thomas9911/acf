use std::iter::{Enumerate, Peekable};
use std::ops::Range;
use std::str::Chars;
use itertools::PeekingNext;

const SPECIAL_CHARS: [char; 5] = ['=', ',', '{', '}', ':'];



fn xd((index, ch): &(usize, char)) -> bool {
    SPECIAL_CHARS.contains(ch)
}

// #[derive(Debug)]
pub struct Tokenizer<'a> {
    data: &'a str,
    // chars: Peekable<Enumerate<Chars<'a>>>,
    chars: Box<dyn Iterator<Item=(usize, char)> + 'a>,
    start: usize,
    stop: bool,
    next_range: Option<Range<usize>>
}

impl<'a> Tokenizer<'a> {
    pub fn new(data: &'a str) -> Self {
        Tokenizer {
            data,
            chars: Box::new(data.chars().enumerate().filter(xd)),
            start: 0,
            stop: false,
            next_range: Some(0..1),
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let range = if let Some(current_range) = self.next_range.take() {
            current_range
        } else if let Some((current_postion, ch)) = self.chars.next() {
            let range = self.start..current_postion;
            self.next_range = Some(current_postion..(current_postion+1));
            self.start = (current_postion+1);
            range
        } else {
            if self.stop {
                return None;
            } else {
                self.stop = true;
                self.start..self.data.len()
            }
        };

        if range.len() > 1 {
            return Some(Token {
                kind: TokenKind::String,
                range,
            });
        }
        let kind = match &self.data[range.clone()] {
            "=" => TokenKind::Assignment,
            "," => TokenKind::ItemSeperator,
            "{" => TokenKind::BracketOpen,
            "}" => TokenKind::BracketClose,
            ":" => TokenKind::KeySeperator,
            "" => return None,
            e => panic!("character left: '{}'", e),
        };

        Some(Token { kind, range })
    }
}

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
    // let data = r#"config1={value: 1, default: 12},config2={DEFAULT: "testing"}"#;
    let data = r#"{value: 1, default: 12}"#;
    let mut tokenizer = Tokenizer::new(data);

    let tokens: Vec<_> = tokenizer.map(|token| {
        (
            token.kind,
            token.range.clone(),
            data[token.range].to_string(),
        )
    }).collect();
    dbg!(tokens);
    panic!("no wrok")
}
