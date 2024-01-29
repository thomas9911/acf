use winnow::ascii::multispace0;
use winnow::combinator::{
    alt, cut_err, delimited, preceded, repeat_till, separated, separated_pair,
};
use winnow::error::{InputError, ParseError};
use winnow::prelude::*;
use winnow::stream::{Located, Location};
use winnow::token::{any, none_of, take_while};

pub mod types;
pub use types::{Map, Range, RangeMap, Seq};

const PARSE_FORMAT: u128 = lexical::format::TOML;
const PARSE_FLOAT_OPTION: lexical::ParseFloatOptions = lexical::ParseFloatOptions::new();
const PARSE_INTEGER_OPTION: lexical::ParseIntegerOptions = lexical::ParseIntegerOptions::new();

#[derive(Debug, PartialEq, Eq)]
pub enum ACF {
    String(Range),
    Integer(Range),
    Float(Range),
    Boolean(Range),
    Seq(Range, Seq<ACF>),
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

const SPECIAL_CHARS: [char; 5] = ['=', ',', '{', '}', ':'];

pub fn parse_integer(x: &str) -> Result<i64, lexical::Error> {
    lexical::parse_with_options::<i64, _, PARSE_FORMAT>(x, &PARSE_INTEGER_OPTION)
}

pub fn parse_float(x: &str) -> Result<f64, lexical::Error> {
    lexical::parse_with_options::<f64, _, PARSE_FORMAT>(x, &PARSE_FLOAT_OPTION)
}

// copied mostly from json winnow example: START

fn string<'s>(input: &mut Located<&'s str>) -> PResult<&'s str, InputError<Located<&'s str>>> {
    preceded(
        '"',
        // `cut_err` transforms an `ErrMode::Backtrack(e)` to `ErrMode::Cut(e)`, signaling to
        // combinators like  `alt` that they should not try other parsers. We were in the
        // right branch (since we found the `"` character) but encountered an error when
        // parsing the string
        cut_err(repeat_till::<_, _, (), _, _, _, _>(0.., character, '"').recognize()),
    )
    .parse_next(input)
}

fn character<'s>(input: &mut Located<&'s str>) -> PResult<char, InputError<Located<&'s str>>> {
    let c = none_of('\"').parse_next(input)?;
    if c == '\\' {
        any.verify(|c| match c {
            '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' => true,
            _ => false,
        })
        .parse_next(input)
    } else {
        Ok(c)
    }
}

// copied mostly from json winnow example: END

fn take_single_primative_parser<'s>(
    input: &mut Located<&'s str>,
) -> PResult<&'s str, InputError<Located<&'s str>>> {
    alt((
        string,
        take_while(1.., |ch: char| {
            !(ch.is_whitespace() || SPECIAL_CHARS.contains(&ch))
        }),
    ))
    .parse_next(input)
}

fn primative_parser<'s>(
    input: &mut Located<&'s str>,
) -> PResult<ACF, InputError<Located<&'s str>>> {
    // alphanumeric1.parse_next(input)
    let start = input.location();
    take_single_primative_parser
        .parse_next(input)
        .map(|matched| {
            let end = input.location();
            let range = start..end;

            match matched {
                "true" | "false" => ACF::Boolean(range),
                x if parse_integer(x).is_ok() => ACF::Integer(range),
                x if parse_float(x).is_ok() => ACF::Float(range),
                _ => ACF::String(range),
            }
        })
}

fn range_parser<'s>(input: &mut Located<&'s str>) -> PResult<Range, InputError<Located<&'s str>>> {
    delimited(ws, primative_parser.map(|x| x.into_range()), ws).parse_next(input)
}

// fn key_parser<'s>(
//     input: &mut Located<&'s str>,
// ) -> PResult<StringKey, InputError<Located<&'s str>>> {
//     // alphanumeric1.parse_next(input)
//     take_while(0.., |ch| !SPECIAL_CHARS.contains(&ch))
//         .map(|s| StringKey::from_ref(s))
//         .parse_next(input)
// }

fn map_item_parser<'s>(
    input: &mut Located<&'s str>,
) -> PResult<(Range, ACF), InputError<Located<&'s str>>> {
    delimited(
        ws,
        separated_pair(range_parser, (ws, ":", ws), primative_parser),
        ws,
    )
    .parse_next(input)
}

fn seq_item_parser<'s>(input: &mut Located<&'s str>) -> PResult<ACF, InputError<Located<&'s str>>> {
    delimited(ws, primative_parser, ws).parse_next(input)
}

fn list_item_parser<'s>(
    input: &mut Located<&'s str>,
) -> PResult<ACF, InputError<Located<&'s str>>> {
    let start = input.location();

    delimited(
        ws,
        alt((
            separated(1.., map_item_parser, (ws, ",", ws)).map(|x: RangeMap<ACF>| either::Left(x)),
            separated(1.., seq_item_parser, (ws, ",", ws)).map(|x: Vec<ACF>| either::Right(x)),
        )),
        ws,
    )
    .parse_next(input)
    .map(|options| {
        let end = input.location();
        let range = start..end;

        match options {
            either::Left(x) => ACF::Map(range, x),
            either::Right(x) => ACF::Seq(range, x),
        }
    })

    // separated(0.., map_item_parser, (ws, ",", ws))
    // .parse_next(input)
    // .map(|x: RangeMap<ACF>| {
    //     let end = input.location();
    //     let range = start..end;
    //     ACF::Map(range, x)
    // })
}

fn composite_parser<'s>(
    input: &mut Located<&'s str>,
) -> PResult<ACF, InputError<Located<&'s str>>> {
    delimited(
        ws,
        delimited((ws, "{", ws), list_item_parser, (ws, "}", ws)),
        ws,
    )
    .parse_next(input)
}

fn value_parser<'s>(input: &mut Located<&'s str>) -> PResult<ACF, InputError<Located<&'s str>>> {
    delimited(ws, alt((composite_parser, primative_parser)), ws).parse_next(input)
}

fn item_parser<'s>(
    input: &mut Located<&'s str>,
) -> PResult<(Range, ACF), InputError<Located<&'s str>>> {
    delimited(
        ws,
        separated_pair(range_parser, (ws, "=", ws), value_parser),
        ws,
    )
    .parse_next(input)
}

fn base_parser<'s>(input: &mut Located<&'s str>) -> PResult<ACF, InputError<Located<&'s str>>> {
    let start = input.location();

    delimited(ws, separated(0.., item_parser, (ws, ",", ws)), ws)
        .parse_next(input)
        .map(|x| {
            let end = input.location();
            let range = start..end;
            ACF::Map(range, x)
        })
}

fn ws<'s>(input: &mut Located<&'s str>) -> PResult<&'s str, InputError<Located<&'s str>>> {
    multispace0.parse_next(input)
}

pub fn tokenize_ast<'s>(
    data: &'s str,
) -> Result<ACF, ParseError<Located<&'s str>, InputError<Located<&'s str>>>> {
    base_parser.parse(Located::new(data))
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
    let out = tokenize_ast(data).unwrap();

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

#[test]
fn tokenize_this2() {
    let data = r#"config1=testing"#;
    let out = tokenize_ast(data).unwrap();

    let mut strings = Vec::new();
    debug_visit_ast(data, &out, &mut strings);
    let expected = vec![('m', "config1=testing"), ('k', "config1"), ('s', "testing")];

    assert_eq!(expected, strings);
}

#[test]
fn tokenize_this3() {
    let data = r#"config1={1,2,3,4,5}"#;
    let out = tokenize_ast(data).unwrap();

    let mut strings = Vec::new();
    debug_visit_ast(data, &out, &mut strings);
    let expected = vec![
        ('m', "config1={1,2,3,4,5}"),
        ('k', "config1"),
        ('l', "1,2,3,4,5"),
        ('i', "1"),
        ('i', "2"),
        ('i', "3"),
        ('i', "4"),
        ('i', "5"),
    ];

    assert_eq!(expected, strings);
}

#[test]
fn escaped_text_quote() {
    let data = r#"config1={a: "extra \"quote\""}"#;

    let out = tokenize_ast(data).unwrap();

    let mut strings = Vec::new();
    debug_visit_ast(data, &out, &mut strings);
    let expected = vec![
        ('m', r#"config1={a: "extra \"quote\""}"#),
        ('k', "config1"),
        ('m', r#"a: "extra \"quote\"""#),
        ('k', "a"),
        ('s', r#""extra \"quote\"""#),
    ];

    assert_eq!(expected, strings);
}

#[test]
fn escaped_text_comma() {
    let data = r#"config1={a: "extra, comma"}"#;

    let out = tokenize_ast(data).unwrap();

    let mut strings = Vec::new();
    debug_visit_ast(data, &out, &mut strings);
    let expected = vec![
        ('m', "config1={a: \"extra, comma\"}"),
        ('k', "config1"),
        ('m', "a: \"extra, comma\""),
        ('k', "a"),
        ('s', "\"extra, comma\""),
    ];

    assert_eq!(expected, strings);
}

#[test]
fn escaped_text_colon() {
    let data = r#"config1={a: "extra: colon"}"#;

    let out = tokenize_ast(data).unwrap();

    let mut strings = Vec::new();
    debug_visit_ast(data, &out, &mut strings);
    let expected = vec![
        ('m', "config1={a: \"extra: colon\"}"),
        ('k', "config1"),
        ('m', "a: \"extra: colon\""),
        ('k', "a"),
        ('s', "\"extra: colon\""),
    ];

    assert_eq!(expected, strings);
}

#[test]
fn escaped_text_bracket() {
    let data = r#"config1={a: "{bracket}"}"#;

    let out = tokenize_ast(data).unwrap();

    let mut strings = Vec::new();
    debug_visit_ast(data, &out, &mut strings);
    let expected = vec![
        ('m', "config1={a: \"{bracket}\"}"),
        ('k', "config1"),
        ('m', "a: \"{bracket}\""),
        ('k', "a"),
        ('s', "\"{bracket}\""),
    ];

    assert_eq!(expected, strings);
}
