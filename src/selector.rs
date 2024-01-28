use crate::ACF;
use kstring::KStringCow;

// pub enum KeyIndex {
//     String(KString),
//     Integer(isize),
// }

// impl From<String> for KeyIndex {
//     fn from(value: String) -> Self {
//         KeyIndex::String(KString::from(value))
//     }
// }

// impl From<&str> for KeyIndex {
//     fn from(value: &str) -> Self {
//         KeyIndex::String(KString::from_ref(value))
//     }
// }

// impl From<&isize> for KeyIndex {
//     fn from(value: &isize) -> Self {
//         KeyIndex::Integer(*value)
//     }
// }

pub fn parse_json_pointer<'a, I: FromIterator<KeyIndexRef<'a>>>(input: &'a str) -> Option<I> {
    if input.is_empty() {
        return Some(I::from_iter([]));
    }
    if !input.starts_with('/') {
        return None;
    }

    let iterator = input
        .split('/')
        .skip(1)
        .map(|x| KStringCow::from(x.replace("~1", "/").replace("~0", "~")))
        .map(|x| match x.parse() {
            Ok(int) if int >= 0 => KeyIndexRef::Integer(int),
            _ => KeyIndexRef::String(x),
        });

    Some(iterator.collect())
}

#[derive(Debug, PartialEq)]
pub enum KeyIndexRef<'a> {
    String(KStringCow<'a>),
    Integer(isize),
}

impl<'a> From<&'a str> for KeyIndexRef<'a> {
    fn from(value: &'a str) -> Self {
        KeyIndexRef::String(KStringCow::from_ref(value))
    }
}

impl<'a> From<isize> for KeyIndexRef<'a> {
    fn from(value: isize) -> Self {
        KeyIndexRef::Integer(value)
    }
}

pub fn selector<'a, 'b, I: IntoIterator<Item = &'b KeyIndexRef<'b>>>(
    config: &'a ACF,
    selector: I,
) -> Option<&'a ACF> {
    let mut config_pointer = config;

    for key in selector {
        match key {
            KeyIndexRef::String(key) => {
                config_pointer = config_pointer.get(key)?;
            }
            KeyIndexRef::Integer(key) => {
                config_pointer = config_pointer.get_index(*key)?;
            }
        }
    }

    Some(config_pointer)
}

#[test]
fn selector_test() {
    use crate::{acf_map, acf_seq};

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
        selector(&config, &["config1".into(), "yes".into()]).unwrap()
    );
    assert_eq!(
        &ACF::from(12),
        selector(&config, &["config1".into(), "default".into()]).unwrap()
    );
    assert_eq!(
        &ACF::from("testing"),
        selector(&config, &["config2".into(), "DEFAULT".into()]).unwrap()
    );
    assert_eq!(
        &ACF::from(false),
        selector(&config, &["config3".into(), 0.into()]).unwrap()
    );
    assert_eq!(
        &ACF::from(1.23),
        selector(&config, &["config3".into(), (-1).into()]).unwrap()
    );
}

#[test]
fn selector_with_json_pointer_test() {
    use crate::{acf_map, acf_seq};

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

    let path: Vec<_> = parse_json_pointer("/config1/yes").unwrap();
    assert_eq!(&ACF::from(true), selector(&config, &path).unwrap());
    let path: Vec<_> = parse_json_pointer("/config1/default").unwrap();
    assert_eq!(&ACF::from(12), selector(&config, &path).unwrap());
    let path: Vec<_> = parse_json_pointer("/config2/DEFAULT").unwrap();
    assert_eq!(&ACF::from("testing"), selector(&config, &path).unwrap());
    let path: Vec<_> = parse_json_pointer("/config3/0").unwrap();
    assert_eq!(&ACF::from(false), selector(&config, &path).unwrap());
    let path: Vec<_> = parse_json_pointer("/config3/2").unwrap();
    assert_eq!(&ACF::from(1.23), selector(&config, &path).unwrap());

    // negative numbers is not allowed in json pointer
    let path: Vec<_> = parse_json_pointer("/config3/-1").unwrap();
    assert!(selector(&config, &path).is_none());
}

#[test]
fn parse_json_pointer_test() {
    let expected: Vec<KeyIndexRef> = vec![
        "a".into(),
        "b".into(),
        "c".into(),
        "d".into(),
        "e".into(),
        "f".into(),
        "g".into(),
        0.into(),
    ];

    let out: Vec<_> = parse_json_pointer("/a/b/c/d/e/f/g/0").unwrap();

    assert_eq!(expected, out);
}

#[test]
fn parse_json_pointer_empty_test() {
    let expected: Vec<KeyIndexRef> = Vec::new();

    let out: Vec<_> = parse_json_pointer("").unwrap();

    assert_eq!(expected, out);
}

#[test]
fn parse_json_pointer_invalid_test() {
    let out: Option<Vec<_>> = parse_json_pointer("abcdefg");
    assert!(out.is_none());
}
