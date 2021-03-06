use crate::error::{Error, Result};
use crate::objects::IndirectRef;
use crate::objects::Object;
use crate::parsing::keywords::OBJ_KEYWORD;
use crate::parsing::tokens::{parse_token, ParseResult, Token};
use std::borrow::Cow;
use std::collections::HashMap;
use std::vec::Drain;

#[derive(Debug, PartialEq)]
pub enum ParseStackEntry<'a> {
    Obj(Object<'a>),
    BeginArray,
    BeginDictionary,
}

use ParseStackEntry::*;

pub struct ParseStack<'a> {
    inner: Vec<ParseStackEntry<'a>>,
}

impl<'a> ParseStack<'a> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn push(&mut self, entry: ParseStackEntry<'a>) {
        self.inner.push(entry)
    }

    pub fn pop(&mut self) -> Option<ParseStackEntry<'a>> {
        self.inner.pop()
    }

    pub fn pop_obj(&mut self) -> Result<Object<'a>> {
        if let Some(Obj(obj)) = self.pop() {
            Ok(obj)
        } else {
            Err(Error::Syntax("Could not pop argument", "".into()))
        }
    }

    pub fn pop_back_to(
        &mut self,
        start_entry: &ParseStackEntry<'a>,
    ) -> Result<Drain<ParseStackEntry<'a>>> {
        // Find the index of the most recent start_entry
        let start = self.inner.len()
            - self
                .inner
                .iter()
                .rev()
                .position(|e| e == start_entry)
                .ok_or(Error::Syntax(
                    "Could not find start of structure",
                    format!("{:?}", start_entry),
                ))?
            - 1;
        // Pop the array elements, in the right order
        let mut entries = self.inner.drain(start..);
        // Skip the starting marker
        entries.next();
        // Return the iterator
        Ok(entries)
    }
}

pub fn parse_object_until_keyword<'a>(
    mut raw: &'a [u8],
    end_keyword: &'static [u8],
) -> ParseResult<'a, (Option<IndirectRef>, Object<'a>)> {
    let mut indirect = None;
    let mut obj_handler = |stack: &mut ParseStack<'a>| -> Result<bool> {
        process_indirect(stack)?;
        if let Object::Indirect(ind) = stack.pop_obj()? {
            indirect = Some(ind);
        } else {
            unreachable!();
        }
        Ok(true)
    };

    let mut object = None;
    let mut end_handler = |stack: &mut ParseStack<'a>| -> Result<bool> {
        object = Some(stack.pop_obj()?);
        Ok(false)
    };

    let mut keyword_handlers: KeywordHandlerMap = HashMap::new();
    keyword_handlers.insert(OBJ_KEYWORD, &mut obj_handler);
    keyword_handlers.insert(end_keyword, &mut end_handler);

    ((), raw) = parse(raw, &mut keyword_handlers)?;

    let object = object.ok_or_else(|| Error::Syntax("Did not encounter end keyword", "".into()))?;
    Ok(((indirect, object), raw))
}

pub type KeywordHandlerMap<'a, 'b> =
    HashMap<&'static [u8], &'b mut (dyn FnMut(&mut ParseStack<'a>) -> Result<bool>)>;

pub fn parse<'a, 'b>(
    mut raw: &'a [u8],
    keyword_handlers: &mut KeywordHandlerMap<'a, 'b>,
) -> ParseResult<'a, ()> {
    let mut stack = ParseStack::new();
    let mut running = true;

    while running {
        let (token, rest) = parse_token(raw)?;
        raw = rest;

        match token {
            // Keyword Handlers
            Token::Keyword(k) if keyword_handlers.contains_key(k) => {
                let handler = keyword_handlers.get_mut(k).unwrap();
                running = handler(&mut stack)?;
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
            Token::EndArray => process_array(&mut stack)?,

            // Dictionary Objects
            Token::BeginDictionary => stack.push(BeginDictionary),
            Token::EndDictionary => process_dictionary(&mut stack)?,

            // Stream Objects
            Token::Stream(stream) => process_stream(&mut stack, stream)?,

            // Null Object
            Token::Keyword(b"null") => stack.push(Obj(Object::Null)),

            // Indirect Objects
            Token::Keyword(b"R") => process_indirect(&mut stack)?,

            // Other
            Token::Keyword(keyword) => {
                return Err(Error::Syntax(
                    "Unrecognized keyword",
                    String::from_utf8_lossy(keyword).into(),
                ))
            }
        }
    }

    Ok(((), raw))
}

fn process_array(stack: &mut ParseStack) -> Result<()> {
    // Pop the array elements, in the right order
    let entries = stack.pop_back_to(&BeginArray)?;
    // Then unwrap them into objects
    let objects = entries
        .map(|entry| {
            if let Obj(object) = entry {
                Ok(object)
            } else {
                Err(Error::Syntax("Unrecognized token inside array", "".into()))
            }
        })
        .collect::<Result<Vec<Object>>>()?;
    // Push an Obj
    stack.push(Obj(Object::Array(objects)));

    Ok(())
}

fn process_dictionary<'a>(stack: &mut ParseStack<'a>) -> Result<()> {
    // Pop the dictionary elements, in the right order
    let mut entries = stack.pop_back_to(&BeginDictionary)?;
    // Then unwrap them into key/value pairs
    let mut dict = HashMap::<Cow<'a, [u8]>, Object<'a>>::with_capacity(entries.len() / 2);
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
    // Push an Obj
    stack.push(Obj(Object::Dictionary(dict)));

    Ok(())
}

fn process_stream<'a>(stack: &mut ParseStack<'a>, stream: &'a [u8]) -> Result<()> {
    let dict = stack.pop_obj()?;
    let mut stream = Cow::Borrowed(stream);

    for filter in &dict[b"Filter"] {
        match filter.as_name()?.as_ref() {
            b"FlateDecode" => {
                stream = inflate::inflate_bytes_zlib(&stream).unwrap().into();
            }
            name => return Err(Error::UnknownFilter(String::from_utf8_lossy(name).into())),
        }
    }

    stack.push(Obj(Object::Stream(dict.into(), stream)));

    Ok(())
}

fn process_indirect(stack: &mut ParseStack) -> Result<()> {
    // The order is reversed as they are being popped from a stack
    let generation = stack.pop_obj()?.as_int()?;
    let number = stack.pop_obj()?.as_int()?;

    stack.push(Obj(Object::Indirect(IndirectRef {
        number: number as u32,
        generation: generation as u16,
    })));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_boolean() {
        let ((_, obj), _raw) = parse_object_until_keyword(b"true end ", b"end").unwrap();
        assert_eq!(obj, Object::Boolean(true));

        let ((_, obj), _raw) = parse_object_until_keyword(b"false end ", b"end").unwrap();
        assert_eq!(obj, Object::Boolean(false));
    }

    #[test]
    fn should_parse_numeric() {
        let ((_, obj), _raw) = parse_object_until_keyword(b"42 end ", b"end").unwrap();
        assert_eq!(obj, Object::Integer(42));

        let ((_, obj), _raw) = parse_object_until_keyword(b"+3.14 end ", b"end").unwrap();
        assert_eq!(obj, Object::Real(3.14));
    }

    #[test]
    fn should_parse_string() {
        let ((_, obj), _raw) = parse_object_until_keyword(b"(Hello, world!) end ", b"end").unwrap();
        assert_eq!(obj, Object::String(Cow::Borrowed(b"Hello, world!")));

        let ((_, obj), _raw) = parse_object_until_keyword(b"<616263> end ", b"end").unwrap();
        assert_eq!(obj, Object::String(Cow::Borrowed(b"abc")));
    }

    #[test]
    fn should_parse_name() {
        let ((_, obj), _raw) = parse_object_until_keyword(b"/Name end ", b"end").unwrap();
        assert_eq!(obj, Object::Name(Cow::Borrowed(b"Name")));
    }

    #[test]
    fn should_parse_array() {
        let ((_, obj), _raw) = parse_object_until_keyword(b"[1 2 3] end ", b"end").unwrap();
        assert_eq!(
            obj,
            Object::Array(vec![
                Object::Integer(1),
                Object::Integer(2),
                Object::Integer(3)
            ])
        );

        let ((_, obj), _raw) = parse_object_until_keyword(b"[1[2]3] end ", b"end").unwrap();
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
        let ((_, obj), _raw) = parse_object_until_keyword(raw, b"end").unwrap();

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
        let ((_, obj), _raw) = parse_object_until_keyword(raw, b"end").unwrap();
        assert_eq!(
            obj,
            Object::Stream(
                Box::new(Object::Dictionary(HashMap::new())),
                Cow::Borrowed(b"Hello, world!\n")
            )
        );
    }

    #[test]
    fn should_parse_null() {
        let ((_, obj), _raw) = parse_object_until_keyword(b"null end ", b"end").unwrap();
        assert_eq!(obj, Object::Null);
    }

    #[test]
    fn should_parse_indirect() {
        let ((_, obj), _raw) = parse_object_until_keyword(b"12 0 R end ", b"end").unwrap();
        assert_eq!(
            obj,
            Object::Indirect(IndirectRef {
                number: 12,
                generation: 0
            })
        );
    }

    #[test]
    fn should_parse_obj_keyword() {
        let ((ind, obj), _raw) =
            parse_object_until_keyword(b"1 2 obj 12 0 R end ", b"end").unwrap();
        assert_eq!(
            ind,
            Some(IndirectRef {
                number: 1,
                generation: 2,
            })
        );
        assert_eq!(
            obj,
            Object::Indirect(IndirectRef {
                number: 12,
                generation: 0
            })
        );
    }
}
