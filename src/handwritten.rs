// use std::iter::{Enumerate, Peekable};
// use std::str::Chars;

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