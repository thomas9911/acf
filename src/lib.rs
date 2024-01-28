use ahash::RandomState;
use indexmap::IndexMap;
use kstring::KString;
use ordered_float::OrderedFloat;
use smallvec::SmallVec;
use snailquote::unescape;

pub type Map<K, V> = IndexMap<K, V, RandomState>;
pub type StringKey = String;
pub type StringMap<V> = Map<StringKey, V>;

pub mod selector;
pub mod token;

use token::{parse_float, parse_integer};

pub use crate::selector::KeyIndexRef;

#[macro_export]
macro_rules! acf_map {
    ($($key:expr => $value:expr,)+) => { acf_map!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            const CAP: usize = <[()]>::len(&[$({ stringify!($key); }),*]);
            let mut map = crate::StringMap::<ACF>::with_capacity_and_hasher(CAP, ahash::RandomState::new());
            $(
                map.insert($key.into(), ACF::from($value));
            )*
            ACF::Map(map)
        }
    };
}

#[macro_export]
macro_rules! acf_seq {
    ($($value:expr,)+) => { acf_seq!($($value),+) };
    ($($value:expr),*) => {
        {
            const CAP: usize = <[()]>::len(&[$({ stringify!($value); }),*]);
            let mut set = Vec::<ACF>::with_capacity(CAP);
            $(
                set.push($value.into());
            )*
            ACF::Seq(set)
        }
    };
}

#[derive(Debug, PartialEq, Eq)]
pub enum ACF {
    String(KString),
    Integer(i64),
    Float(OrderedFloat<f64>),
    Boolean(bool),
    Seq(Vec<ACF>),
    Map(StringMap<ACF>),
}

impl From<String> for ACF {
    fn from(value: String) -> Self {
        ACF::String(KString::from(value))
    }
}

impl From<&str> for ACF {
    fn from(value: &str) -> Self {
        ACF::String(KString::from_ref(value))
    }
}

impl From<bool> for ACF {
    fn from(value: bool) -> Self {
        ACF::Boolean(value)
    }
}

impl From<i64> for ACF {
    fn from(value: i64) -> Self {
        ACF::Integer(value)
    }
}

impl From<f64> for ACF {
    fn from(value: f64) -> Self {
        ACF::Float(OrderedFloat::from(value))
    }
}

impl<T> From<Vec<T>> for ACF
where
    T: Into<ACF>,
{
    fn from(value: Vec<T>) -> Self {
        ACF::Seq(value.into_iter().map(|x| x.into()).collect())
    }
}

impl<T> From<&[T]> for ACF
where
    T: Into<ACF> + Clone,
{
    fn from(value: &[T]) -> Self {
        ACF::Seq(value.into_iter().map(|x| x.clone().into()).collect())
    }
}

impl From<StringMap<ACF>> for ACF {
    fn from(value: StringMap<ACF>) -> Self {
        ACF::Map(value)
    }
}

impl ACF {
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            ACF::Map(map) => map.get(key),
            _ => None,
        }
    }

    pub fn get_index(&self, index: isize) -> Option<&Self> {
        match self {
            ACF::Seq(vector) if index >= 0 => vector.get(index as usize),
            ACF::Seq(vector)
                if index < 0
                    && vector
                        .len()
                        .checked_sub(index.checked_abs()? as usize)
                        .is_some() =>
            {
                vector.get(vector.len() - (-index) as usize)
            }
            _ => None,
        }
    }

    pub fn selector(&self, selector: &[KeyIndexRef<'_>]) -> Option<&Self> {
        selector::selector(self, selector)
    }

    pub fn json_pointer(&self, pointer: &str) -> Option<&Self> {
        let mut config_pointer = self;

        for key in selector::parse_json_pointer::<SmallVec<[_; 8]>>(pointer)? {
            match key {
                KeyIndexRef::String(key) => {
                    config_pointer = config_pointer.get(&key)?;
                }
                KeyIndexRef::Integer(key) => {
                    config_pointer = config_pointer.get_index(key)?;
                }
            }
        }

        Some(config_pointer)
    }
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
        token::ACF::String(range) => {
            ACF::String(unescape(&input[range]).unwrap_or(String::new()).into())
        }
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

#[test]
fn parse_config() {
    let mut data = r#"
    config1={value: 1, default: 12, yes: true},
    config2={DEFAULT: "testing", extra: "extra \'quotes\'"},
    config3={false, 123, 1.23}
    "#;
    let tokens = token::tokenize_ast(&mut data).unwrap();

    let out = tokenized_to_config(data, tokens);

    let expected = acf_map! {
        "config1" => acf_map! {
            "value" => 1,
            "default" => 12,
            "yes" => true,
        },
        "config2" => acf_map! {
            "DEFAULT" => "testing",
            "extra" => "extra 'quotes'",
        },
        "config3" => acf_seq!{false, 123, 1.23}
    };

    assert_eq!(out, expected);
}

#[test]
fn acf_key_index_bounds() {
    let data = acf_seq! {1, 2};

    assert!(data.get_index(isize::MAX).is_none());
    assert!(data.get_index(isize::MIN).is_none());
    assert!(data.get_index(0).is_some());
    assert!(data.get_index(1).is_some());
    assert!(data.get_index(-1).is_some());
    assert!(data.get_index(-2).is_some());
}

#[test]
fn acf_json_pointer() {
    let config = acf_map! {
        "config1" => acf_map! {
            "value" => 1,
            "default" => 12,
            "yes" => true,
        },
        "config2" => acf_map! {
            "DEFAULT" => "testing",
            "extra" => "extra 'quotes'",
        },
        "config3" => acf_seq!{false, 123, 1.23}
    };

    assert_eq!(
        &ACF::from(true),
        config.json_pointer("/config1/yes").unwrap()
    );
    assert_eq!(
        &ACF::from(12),
        config.json_pointer("/config1/default").unwrap()
    );
    assert_eq!(
        &ACF::from("testing"),
        config.json_pointer("/config2/DEFAULT").unwrap()
    );
    assert_eq!(
        &ACF::from(false),
        config.json_pointer("/config3/0").unwrap()
    );
    assert_eq!(&ACF::from(1.23), config.json_pointer("/config3/2").unwrap());
}
