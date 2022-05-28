use crate::error::Error;
use crate::tokens::{parse_token, ParseResult, Token};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Index;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct IndirectRef {
    pub number: u32,
    pub generation: u16,
}

#[derive(Debug, PartialEq)]
pub enum Object<'a> {
    Boolean(bool),
    Integer(usize),
    Real(f64),
    String(Cow<'a, [u8]>),
    Name(Cow<'a, [u8]>),
    Array(Vec<Object<'a>>),
    Dictionary(HashMap<Cow<'a, [u8]>, Object<'a>>),
    Stream(HashMap<Cow<'a, [u8]>, Object<'a>>, &'a [u8]),
    Null,
    Indirect(IndirectRef),
}

impl<'a> Index<&'a [u8]> for Object<'a> {
    type Output = Object<'a>;

    fn index(&self, index: &'a [u8]) -> &Object<'a> {
        if let Object::Dictionary(dict) = self {
            dict.get(&Cow::Borrowed(index)).unwrap_or(&Object::Null)
        } else {
            &Object::Null
        }
    }
}

#[derive(Debug, PartialEq)]
enum ParseStackEntry<'a> {
    Obj(Object<'a>),
    BeginArray,
    BeginDictionary,
}

pub fn parse_object_until_keyword<'a>(
    mut raw: &'a [u8],
    end_keyword: &[u8],
) -> ParseResult<'a, Object<'a>> {
    use ParseStackEntry::*;
    let mut stack = Vec::new();

    loop {
        // TODO: Remove once tested
        eprintln!("Parse stack: {:?}", stack);
        let (token, rest) = parse_token(raw)?;
        raw = rest;

        match token {
            // End Keyword
            Token::Keyword(k) if k == end_keyword => {
                if let Some(Obj(object)) = stack.into_iter().next() {
                    return Ok((object, raw));
                } else {
                    return Err(Error::Syntax(
                        "Encountered end keyword without reading a full object",
                        "".into(),
                    ));
                };
            }

            // Boolean Objects
            Token::Keyword(b"true") => stack.push(Obj(Object::Boolean(true))),
            Token::Keyword(b"false") => stack.push(Obj(Object::Boolean(false))),

            // Numeric Objects
            Token::Integer(i) => stack.push(Obj(Object::Integer(i))),
            Token::Real(x) => stack.push(Obj(Object::Real(x))),

            // String Objects
            Token::LiteralString(s) => stack.push(Obj(Object::String(s))),
            Token::HexadecimalString(s) => stack.push(Obj(Object::String(s))),

            // Name Objects
            Token::Name(n) => stack.push(Obj(Object::Name(n))),

            // Array Objects
            Token::BeginArray => stack.push(BeginArray),
            Token::EndArray => {
                // Find the index of the most recent BeginArray
                let start = stack.len()
                    - stack
                        .iter()
                        .rev()
                        .position(|e| e == &BeginArray)
                        .ok_or(Error::Syntax("Could not find start of array", "".into()))?;
                // Pop the array elements, in the right order
                let entries = stack.drain(start..);
                // Then unwrap them into objects
                let mut array = Vec::with_capacity(entries.len());
                for entry in entries {
                    if let Obj(object) = entry {
                        array.push(object);
                    } else {
                        return Err(Error::Syntax("Unrecognized token inside array", "".into()));
                    }
                }
                // Pop the BeginArray
                stack.pop();
                // Push an Obj
                stack.push(Obj(Object::Array(array)));
            }

            // Dictionary Objects
            Token::BeginDictionary => stack.push(BeginDictionary),
            Token::EndDictionary => {
                // Find the index of the most recent BeginDictionary
                let start = stack.len()
                    - stack
                        .iter()
                        .rev()
                        .position(|e| e == &BeginDictionary)
                        .ok_or(Error::Syntax(
                            "Could not find start of dictionary",
                            format!("{:?}", stack),
                        ))?;
                // Pop the dictionary elements, in the right order
                let mut entries = stack.drain(start..);
                // Then unwrap them into key/value pairs
                let mut dict =
                    HashMap::<Cow<'a, [u8]>, Object<'a>>::with_capacity(entries.len() / 2);
                while let Some(entry) = entries.next() {
                    let key = if let Obj(Object::Name(key)) = entry {
                        key
                    } else {
                        return Err(Error::Syntax(
                            "Misplaced token inside dictionary",
                            format!("{:?}", entry),
                        ));
                    };

                    let value = if let Some(Obj(value)) = entries.next() {
                        value
                    } else {
                        return Err(Error::Syntax(
                            "Could not find value in dictionary",
                            "".into(),
                        ));
                    };

                    dict.insert(key, value);
                }
                // Explicitly drop entries so that we can mutate the stack again
                // This was not required for array parsing as there it was
                // consumed by the for loop
                drop(entries);
                // Pop the BeginDictionary
                stack.pop();
                // Push an Obj
                stack.push(Obj(Object::Dictionary(dict)));
            }

            // Stream Objects
            Token::Stream(stream) => {
                if let Some(Obj(Object::Dictionary(dict))) = stack.pop() {
                    stack.push(Obj(Object::Stream(dict, stream)));
                } else {
                    return Err(Error::Syntax("Could not find stream dictionary", "".into()));
                }
            }

            // Null Object
            Token::Keyword(b"null") => stack.push(Obj(Object::Null)),

            // Indirect Objects
            Token::Keyword(b"R") => {
                // The order is reversed as they are being popped from a stack
                if let (
                    Some(Obj(Object::Integer(generation))),
                    Some(Obj(Object::Integer(number))),
                ) = (stack.pop(), stack.pop())
                {
                    // TODO: error handling for integer casts?
                    stack.push(Obj(Object::Indirect(IndirectRef {
                        number: number as u32,
                        generation: generation as u16,
                    })));
                } else {
                    return Err(Error::Syntax(
                        "Could not find integers for indirect object",
                        "".into(),
                    ));
                }
            }

            // Other
            Token::Keyword(keyword) => {
                return Err(Error::Syntax(
                    "Unrecognized keyword",
                    String::from_utf8_lossy(keyword).into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_index_into_dictionary() {
        let dict = Object::Dictionary({
            let mut dict = HashMap::<Cow<[u8]>, Object>::new();
            dict.insert(Cow::Borrowed(b"Name"), Object::Boolean(true));
            dict
        });

        assert_eq!(dict[b"Name"], Object::Boolean(true));
        assert_eq!(dict[b"NotFound"], Object::Null);
        assert_eq!(Object::Null[b"NotFound"], Object::Null);
    }

    #[test]
    fn should_parse_boolean() {
        let (obj, _raw) = parse_object_until_keyword(b"true end ", b"end").unwrap();
        assert_eq!(obj, Object::Boolean(true));

        let (obj, _raw) = parse_object_until_keyword(b"false end ", b"end").unwrap();
        assert_eq!(obj, Object::Boolean(false));
    }

    #[test]
    fn should_parse_numeric() {
        let (obj, _raw) = parse_object_until_keyword(b"42 end ", b"end").unwrap();
        assert_eq!(obj, Object::Integer(42));

        let (obj, _raw) = parse_object_until_keyword(b"+3.14 end ", b"end").unwrap();
        assert_eq!(obj, Object::Real(3.14));
    }

    #[test]
    fn should_parse_string() {
        let (obj, _raw) = parse_object_until_keyword(b"(Hello, world!) end ", b"end").unwrap();
        assert_eq!(obj, Object::String(Cow::Borrowed(b"Hello, world!")));

        let (obj, _raw) = parse_object_until_keyword(b"<616263> end ", b"end").unwrap();
        assert_eq!(obj, Object::String(Cow::Borrowed(b"abc")));
    }

    #[test]
    fn should_parse_name() {
        let (obj, _raw) = parse_object_until_keyword(b"/Name end ", b"end").unwrap();
        assert_eq!(obj, Object::Name(Cow::Borrowed(b"Name")));
    }

    #[test]
    fn should_parse_array() {
        let (obj, _raw) = parse_object_until_keyword(b"[1 2 3] end ", b"end").unwrap();
        assert_eq!(
            obj,
            Object::Array(vec![
                Object::Integer(1),
                Object::Integer(2),
                Object::Integer(3)
            ])
        );

        let (obj, _raw) = parse_object_until_keyword(b"[1[2]3] end ", b"end").unwrap();
        assert_eq!(
            obj,
            Object::Array(vec![
                Object::Integer(1),
                Object::Array(vec![Object::Integer(2)]),
                Object::Integer(3)
            ])
        );
    }

    #[test]
    fn should_parse_dictionary() {
        // Example from Adobe (2008, p. 18)
        let raw = b"<< /Type /Example
                       /Subtype /DictionaryExample
                       /Version 0.01
                       /IntegerItem 12
                       /StringItem (a string)
                       /Subdictionary << /Item1 0.4
                                         /Item2 true
                                         /LastItem (not!)
                                         /VeryLastItem (OK)
                                      >>
                    >> end ";
        let (obj, _raw) = parse_object_until_keyword(raw, b"end").unwrap();

        assert_eq!(obj[b"Type"], Object::Name(Cow::Borrowed(b"Example")));
        assert_eq!(
            obj[b"Subtype"],
            Object::Name(Cow::Borrowed(b"DictionaryExample"))
        );
        assert_eq!(obj[b"Version"], Object::Real(0.01));
        assert_eq!(obj[b"IntegerItem"], Object::Integer(12));
        assert_eq!(
            obj[b"StringItem"],
            Object::String(Cow::Borrowed(b"a string"))
        );

        let subdict = &obj[b"Subdictionary"];
        assert_eq!(subdict[b"Item1"], Object::Real(0.4));
        assert_eq!(subdict[b"Item2"], Object::Boolean(true));
        assert_eq!(subdict[b"LastItem"], Object::String(Cow::Borrowed(b"not!")));
        assert_eq!(
            subdict[b"VeryLastItem"],
            Object::String(Cow::Borrowed(b"OK"))
        );
    }

    #[test]
    fn should_parse_stream() {
        let raw = b"<< >> stream\nHello, world!\nendstream end ";
        let (obj, _raw) = parse_object_until_keyword(raw, b"end").unwrap();
        assert_eq!(obj, Object::Stream(HashMap::new(), b"Hello, world!\n"));
    }

    #[test]
    fn should_parse_null() {
        let (obj, _raw) = parse_object_until_keyword(b"null end ", b"end").unwrap();
        assert_eq!(obj, Object::Null);
    }

    #[test]
    fn should_parse_indirect() {
        let (obj, _raw) = parse_object_until_keyword(b"12 0 R end ", b"end").unwrap();
        assert_eq!(
            obj,
            Object::Indirect(IndirectRef {
                number: 12,
                generation: 0
            })
        );
    }
}
