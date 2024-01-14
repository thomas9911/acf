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

pub type Map<K, V> = IndexMapWrapper<K, V, RandomState>;
pub type StringKey = KString;
pub type StringMap<V> = Map<StringKey, V>;

#[derive(Debug, PartialEq, Eq)]
pub enum ACF {
    String(String),
    Integer(i64),
    Float(OrderedFloat<f64>),
    Seq(Vec<ACF>),
    Map(StringMap<ACF>),
}

#[derive(Debug)]
pub struct IndexMapWrapper<
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: std::hash::BuildHasher,
>(IndexMap<K, V, H>);

impl<K, V, H> PartialEq for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: std::hash::BuildHasher,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K, V, H> Eq for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: std::hash::BuildHasher,
{
}

impl<K, V, H> std::ops::Deref for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: std::hash::BuildHasher,
{
    type Target = IndexMap<K, V, H>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V, H> std::ops::DerefMut for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: std::hash::BuildHasher,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V, H> FromIterator<(K, V)> for IndexMapWrapper<K, V, H>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
    H: std::hash::BuildHasher + Default,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        IndexMapWrapper(IndexMap::from_iter(iterable))
    }
}

const SPECIAL_CHARS: [char; 5] = ['=', ',', '{', '}', ':'];

impl<K: std::hash::Hash, V: std::cmp::Eq, S> Accumulate<(K, V)> for IndexMapWrapper<K, V, S>
where
    K: std::cmp::Eq + std::hash::Hash,
    S: BuildHasher + Default,
{
    #[inline(always)]
    fn initial(capacity: Option<usize>) -> Self {
        let h = S::default();
        match capacity {
            Some(capacity) => IndexMapWrapper(IndexMap::with_capacity_and_hasher(capacity, h)),
            None => IndexMapWrapper(IndexMap::with_hasher(h)),
        }
    }
    #[inline(always)]
    fn accumulate(&mut self, (key, value): (K, V)) {
        self.insert(key, value);
    }
}

fn value_parser<'s>(input: &mut &'s str) -> PResult<&'s str, InputError<&'s str>> {
    // alphanumeric1.parse_next(input)
    take_while(0.., |ch| !SPECIAL_CHARS.contains(&ch)).parse_next(input)
}

fn key_parser<'s>(input: &mut &'s str) -> PResult<StringKey, InputError<&'s str>> {
    // alphanumeric1.parse_next(input)
    take_while(0.., |ch| !SPECIAL_CHARS.contains(&ch))
        .map(|s| StringKey::from_ref(s))
        .parse_next(input)
}

fn parser4<'s>(input: &mut &'s str) -> PResult<(StringKey, &'s str), InputError<&'s str>> {
    separated_pair(key_parser, (ws, ":", ws), value_parser).parse_next(input)
}

fn parser3<'s, O: Accumulate<(StringKey, &'s str)>>(
    input: &mut &'s str,
) -> PResult<O, InputError<&'s str>> {
    separated(0.., parser4, (ws, ",", ws)).parse_next(input)
}

fn parser2<'s, O: Accumulate<(StringKey, &'s str)>>(
    input: &mut &'s str,
) -> PResult<O, InputError<&'s str>> {
    delimited(("{", ws), parser3, (ws, "}", ws)).parse_next(input)
}

fn parser1<'s, O: Accumulate<(StringKey, &'s str)>>(
    input: &mut &'s str,
) -> PResult<(StringKey, O), InputError<&'s str>> {
    separated_pair(key_parser, (ws, "=", ws), parser2).parse_next(input)
}

fn parser0<'s, P: Accumulate<(StringKey, &'s str)>, O: Accumulate<(StringKey, P)>>(
    input: &mut &'s str,
) -> PResult<O, InputError<&'s str>> {
    separated(0.., parser1, (ws, ",", ws)).parse_next(input)
}

const WS: &[char] = &[' ', '\t', '\r', '\n'];

fn ws<'s>(input: &mut &'s str) -> PResult<&'s str, InputError<&'s str>> {
    take_while(0.., WS).parse_next(input)
}

pub fn parse_vec<'s>(
    data: &mut &'s str,
) -> Result<Vec<(StringKey, Vec<(StringKey, &'s str)>)>, ParseError<&'s str, InputError<&'s str>>> {
    parser0.parse(data)
}

pub fn parse_map<'s>(
    data: &mut &'s str,
) -> Result<StringMap<StringMap<&'s str>>, ParseError<&'s str, InputError<&'s str>>> {
    parser0.parse(data)
}

#[test]
fn tokenize_this() {
    let mut data = r#"config1={value: 1, default: 12, yes: true},config2={DEFAULT: "testing"}"#;
    // let mut data = r#"config1={value: 1, default: 12}"#;
    let out = parse_map(&mut data).unwrap();
    let expected: StringMap<_> = vec![
        (
            StringKey::from("config1"),
            vec![
                (StringKey::from("value"), "1"),
                (StringKey::from("default"), "12"),
                (StringKey::from("yes"), "true"),
            ]
            .into_iter()
            .collect(),
        ),
        (
            StringKey::from("config2"),
            vec![(StringKey::from("DEFAULT"), "\"testing\"")]
                .into_iter()
                .collect(),
        ),
    ]
    .into_iter()
    .collect();

    assert_eq!(out, expected);
}
