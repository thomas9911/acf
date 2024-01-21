use std::hash::BuildHasher;

// use winnow::ascii::alphanumeric1;
use winnow::prelude::*;
use winnow::stream::Located;
use winnow::token::take_while;
use winnow::{
    combinator::{delimited, separated, separated_pair},
    stream::Accumulate,
};
use winnow::{
    error::{InputError, ParseError},
    stream::Location,
};

use ahash::RandomState;
use indexmap::IndexMap;
use kstring::KString;
use ordered_float::OrderedFloat;

pub type Map<K, V> = IndexMapWrapper<K, V, RandomState>;
pub type StringKey = KString;
pub type StringMap<V> = Map<StringKey, V>;
pub type Range = std::ops::Range<usize>;
pub type RangeMap<V> = Map<Range, V>;

const PARSE_FORMAT: u128 = lexical::format::TOML;
const PARSE_FLOAT_OPTION: lexical::ParseFloatOptions = lexical::ParseFloatOptions::new();
const PARSE_INTEGER_OPTION: lexical::ParseIntegerOptions = lexical::ParseIntegerOptions::new();

#[derive(Debug, PartialEq, Eq)]
pub enum ACF {
    String(Range),
    Integer(Range),
    Float(Range),
    Boolean(Range),
    Seq(Range, Vec<ACF>),
    Map(Range, RangeMap<ACF>),
}

impl ACF {
    pub fn as_range(&self) -> &Range {
        match self {
            ACF::String(range) => range,
            ACF::Integer(range) => range,
            ACF::Float(range) => range,
            ACF::Boolean(range) => range,
            ACF::Seq(range, _) => range,
            ACF::Map(range, _) => range,
        }
    }

    pub fn into_range(self) -> Range {
        match self {
            ACF::String(range) => range,
            ACF::Integer(range) => range,
            ACF::Float(range) => range,
            ACF::Boolean(range) => range,
            ACF::Seq(range, _) => range,
            ACF::Map(range, _) => range,
        }
    }
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

impl<K, V, S> Accumulate<(K, V)> for IndexMapWrapper<K, V, S>
where
    K: std::hash::Hash + std::cmp::Eq,
    V: std::cmp::Eq,
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

fn primative_parser<'s>(
    input: &mut Located<&'s str>,
) -> PResult<ACF, InputError<Located<&'s str>>> {
    // alphanumeric1.parse_next(input)
    let start = input.location();
    take_while(0.., |ch| !SPECIAL_CHARS.contains(&ch))
        .parse_next(input)
        .map(|matched| {
            let end = input.location();
            let range = start..end;

            match matched {
                "true" | "false" => ACF::Boolean(range),
                x if lexical::parse_with_options::<i64, _, PARSE_FORMAT>(
                    x,
                    &PARSE_INTEGER_OPTION,
                )
                .is_ok() =>
                {
                    ACF::Integer(range)
                }
                x if lexical::parse_with_options::<f64, _, PARSE_FORMAT>(
                    x,
                    &PARSE_FLOAT_OPTION,
                )
                .is_ok() =>
                {
                    ACF::Float(range)
                }
                _ => ACF::String(range),
            }
        })
}

fn range_parser<'s>(input: &mut Located<&'s str>) -> PResult<Range, InputError<Located<&'s str>>> {
    primative_parser(input).map(|x| x.into_range())
}

fn key_parser<'s>(
    input: &mut Located<&'s str>,
) -> PResult<StringKey, InputError<Located<&'s str>>> {
    // alphanumeric1.parse_next(input)
    take_while(0.., |ch| !SPECIAL_CHARS.contains(&ch))
        .map(|s| StringKey::from_ref(s))
        .parse_next(input)
}

fn parser4<'s>(
    input: &mut Located<&'s str>,
) -> PResult<(Range, ACF), InputError<Located<&'s str>>> {
    separated_pair(range_parser, (ws, ":", ws), primative_parser).parse_next(input)
}

fn parser3<'s>(input: &mut Located<&'s str>) -> PResult<ACF, InputError<Located<&'s str>>> {
    let start = input.location();

    separated(0.., parser4, (ws, ",", ws))
        .parse_next(input)
        .map(|x: RangeMap<ACF>| {
            let end = input.location();
            let range = start..end;
            ACF::Map(range, x)
        })
}

fn parser2<'s>(input: &mut Located<&'s str>) -> PResult<ACF, InputError<Located<&'s str>>> {
    delimited(("{", ws), parser3, (ws, "}", ws)).parse_next(input)
}

fn parser1<'s>(
    input: &mut Located<&'s str>,
) -> PResult<(Range, ACF), InputError<Located<&'s str>>> {
    separated_pair(range_parser, (ws, "=", ws), parser2).parse_next(input)
}

fn parser0<'s>(input: &mut Located<&'s str>) -> PResult<ACF, InputError<Located<&'s str>>> {
    let start = input.location();

    separated(0.., parser1, (ws, ",", ws))
        .parse_next(input)
        .map(|x: RangeMap<ACF>| {
            let end = input.location();
            let range = start..end;
            ACF::Map(range, x)
        })
}

const WS: &[char] = &[' ', '\t', '\r', '\n'];

fn ws<'s>(input: &mut Located<&'s str>) -> PResult<&'s str, InputError<Located<&'s str>>> {
    take_while(0.., WS).parse_next(input)
}

// pub fn parse_vec<'s>(
//     data: &mut Located<&'s str>,
// ) -> Result<Vec<(StringKey, Vec<(StringKey, Located<&'s str>)>)>, ParseError<Located<&'s str>, InputError<Located<&'s str>>>> {
//     parser0.parse(data)
// }

pub fn tokenize_ast<'s>(
    data: Located<&'s str>,
) -> Result<ACF, ParseError<Located<&'s str>, InputError<Located<&'s str>>>> {
    parser0.parse(data)
}

#[cfg(test)]
fn debug_visit_ast<'a>(input: &'a str, acf: &ACF, out: &mut Vec<(char, &'a str)>) {
    match acf {
        ACF::String(range) => {
            out.push(('s', &input[range.clone()]));
        }
        ACF::Integer(range) => {
            out.push(('i', &input[range.clone()]));
        }
        ACF::Float(range) => {
            out.push(('f', &input[range.clone()]));
        }
        ACF::Boolean(range) => {
            out.push(('b', &input[range.clone()]));
        }
        ACF::Seq(range, rest) => {
            out.push(('l', &input[range.clone()]));
            for item in rest.iter() {
                debug_visit_ast(input, item, out);
            }
        }
        ACF::Map(range, rest) => {
            out.push(('m', &input[range.clone()]));
            for (key, item) in rest.iter() {
                out.push(('k', &input[key.clone()]));
                debug_visit_ast(input, item, out);
            }
        }
    };
}

#[test]
fn tokenize_this() {
    let data =
        r#"config1={value: 1, default: 1_2, yes: true, number: 1.23},config2={DEFAULT: "testing"}"#;
    // let mut data = r#"config1={value: 1, default: 12}"#;
    let xd = Located::new(data);
    let out = tokenize_ast(xd).unwrap();
    // let expected: StringMap<_> = vec![
    //     (
    //         StringKey::from("config1"),
    //         vec![
    //             (StringKey::from("value"), "1"),
    //             (StringKey::from("default"), "12"),
    //             (StringKey::from("yes"), "true"),
    //         ]
    //         .into_iter()
    //         .collect(),
    //     ),
    //     (
    //         StringKey::from("config2"),
    //         vec![(StringKey::from("DEFAULT"), "\"testing\"")]
    //             .into_iter()
    //             .collect(),
    //     ),
    // ]
    // .into_iter()
    // .collect();

    // assert_eq!(out, expected);

    let mut strings = Vec::new();
    debug_visit_ast(data, &out, &mut strings);

    let expected = vec![
    (
        'm',
        "config1={value: 1, default: 1_2, yes: true, number: 1.23},config2={DEFAULT: \"testing\"}",
    ),
    (
        'k',
        "config1",
    ),
    (
        'm',
        "value: 1, default: 1_2, yes: true, number: 1.23",
    ),
    (
        'k',
        "value",
    ),
    (
        'i',
        "1",
    ),
    (
        'k',
        "default",
    ),
    (
        'i',
        "1_2",
    ),
    (
        'k',
        "yes",
    ),
    (
        'b',
        "true",
    ),
    (
        'k',
        "number",
    ),
    (
        'f',
        "1.23",
    ),
    (
        'k',
        "config2",
    ),
    (
        'm',
        "DEFAULT: \"testing\"",
    ),
    (
        'k',
        "DEFAULT",
    ),
    (
        's',
        "\"testing\"",
    ),
];

    assert_eq!(expected, strings);
}
