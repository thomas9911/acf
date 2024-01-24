use std::hash::BuildHasher;

// use winnow::ascii::alphanumeric1;
use winnow::error::{InputError, ParseError};
use winnow::prelude::*;
use winnow::token::take_while;
use winnow::{
    combinator::{delimited, separated, separated_pair},
    stream::Accumulate,
};

use ahash::RandomState;
use indexmap::IndexMap;
use kstring::KString;
use ordered_float::OrderedFloat;
use snailquote::unescape;

pub type Map<K, V> = IndexMap<K, V, RandomState>;
pub type StringKey = String;
pub type StringMap<V> = Map<StringKey, V>;

pub mod token;

use token::{parse_float, parse_integer};

#[derive(Debug, PartialEq, Eq)]
pub enum ACF {
    String(String),
    Integer(i64),
    Float(OrderedFloat<f64>),
    Boolean(bool),
    Seq(Vec<ACF>),
    Map(StringMap<ACF>),
}

pub fn tokenized_to_config(input: &str, tokens: token::ACF) -> ACF {
    match tokens {
        token::ACF::Boolean(range) => ACF::Boolean(to_boolean(&input[range])),
        token::ACF::Integer(range) => {
            ACF::Integer(parse_integer(&input[range]).expect("tokenizer checked this"))
        }
        token::ACF::Float(range) => ACF::Float(OrderedFloat::from(
            parse_float(&input[range]).expect("tokenizer checked this"),
        )),
        token::ACF::String(range) => ACF::String(unescape(&input[range]).unwrap_or(String::new())),
        token::ACF::Seq(_, values) => ACF::Seq(
            values
                .into_iter()
                .map(|value| tokenized_to_config(input, value))
                .collect(),
        ),
        token::ACF::Map(_, map_values) => ACF::Map(
            map_values
                .into_iter()
                .map(|(key, value)| {
                    (
                        unescape(&input[key]).unwrap_or(String::new()),
                        tokenized_to_config(input, value),
                    )
                })
                .collect(),
        ),
    }
}

fn to_boolean(input: &str) -> bool {
    match input {
        "true" => true,
        "false" => false,
        _ => unreachable!(),
    }
}

#[cfg(test)]
macro_rules! indexmap {
    ($($key:expr => $value:expr,)+) => { indexmap!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            // Note: `stringify!($key)` is just here to consume the repetition,
            // but we throw away that string literal during constant evaluation.
            const CAP: usize = <[()]>::len(&[$({ stringify!($key); }),*]);
            let mut map = IndexMap::<_, _, RandomState>::with_capacity_and_hasher(CAP, RandomState::new());
            $(
                map.insert($key, $value);
            )*
            map
        }
    };
}

#[test]
fn parse_config() {
    let mut data = r#"config1={value: 1, default: 12, yes: true},config2={DEFAULT: "testing", extra: "extra \'quotes\'"}"#;
    let tokens = token::tokenize_ast(&mut data).unwrap();

    let out = tokenized_to_config(data, tokens);

    let expected = ACF::Map(indexmap! {
        StringKey::from("config1") => ACF::Map(indexmap! {
            StringKey::from("value") => ACF::Integer(1),
            StringKey::from("default") => ACF::Integer(12),
            StringKey::from("yes") => ACF::Boolean(true),
        }),
        StringKey::from("config2") => ACF::Map(indexmap! {
            StringKey::from("DEFAULT") => ACF::String("testing".to_string()),
            StringKey::from("extra") => ACF::String("extra 'quotes'".to_string()),
        }),
    });

    assert_eq!(out, expected);
}
