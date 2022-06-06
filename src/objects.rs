use crate::error::{Error, Result};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Index;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct IndirectRef {
    pub number: u32,
    pub generation: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Object<'a> {
    Boolean(bool),
    Integer(usize),
    Real(f64),
    String(Cow<'a, [u8]>),
    Name(Cow<'a, [u8]>),
    Array(Vec<Object<'a>>),
    Dictionary(HashMap<Cow<'a, [u8]>, Object<'a>>),
    Stream(Box<Object<'a>>, Cow<'a, [u8]>),
    Null,
    Indirect(IndirectRef),
}

impl<'a> Object<'a> {
    pub fn as_bool(&self) -> Result<bool> {
        if let Object::Boolean(boolean) = self {
            Ok(*boolean)
        } else {
            Err(Error::Syntax("Expected bool", format!("got {:?}", self)))
        }
    }

    pub fn as_int(&self) -> Result<usize> {
        if let Object::Integer(int) = self {
            Ok(*int)
        } else {
            Err(Error::Syntax("Expected int", format!("got {:?}", self)))
        }
    }

    pub fn as_real(&self) -> Result<f64> {
        if let Object::Real(real) = self {
            Ok(*real)
        } else {
            Err(Error::Syntax("Expected real", format!("got {:?}", self)))
        }
    }

    pub fn as_string(&'a self) -> Result<Cow<'a, [u8]>> {
        if let Object::String(string) = self {
            Ok(Cow::Borrowed(&string))
        } else {
            Err(Error::Syntax("Expected string", format!("got {:?}", self)))
        }
    }

    pub fn as_name(&'a self) -> Result<Cow<'a, [u8]>> {
        if let Object::Name(name) = self {
            Ok(Cow::Borrowed(&name))
        } else {
            Err(Error::Syntax("Expected name", format!("got {:?}", self)))
        }
    }

    pub fn as_array(&'a self) -> Result<&'a [Object<'a>]> {
        if let Object::Array(array) = self {
            Ok(array)
        } else {
            Err(Error::Syntax("Expected array", format!("got {:?}", self)))
        }
    }

    pub fn as_dict(&'a self) -> Result<&'a HashMap<Cow<'a, [u8]>, Object<'a>>> {
        if let Object::Dictionary(dict) = self {
            Ok(dict)
        } else {
            Err(Error::Syntax("Expected dict", format!("got {:?}", self)))
        }
    }

    pub fn as_stream(&'a self) -> Result<(&'a HashMap<Cow<'a, [u8]>, Object<'a>>, Cow<'a, [u8]>)> {
        if let Object::Stream(dict, stream) = self {
            Ok((dict.as_dict()?, Cow::Borrowed(stream)))
        } else {
            Err(Error::Syntax("Expected stream", format!("got {:?}", self)))
        }
    }

    pub fn as_null(&'a self) -> Result<()> {
        if let Object::Null = self {
            Ok(())
        } else {
            Err(Error::Syntax("Expected null", format!("got {:?}", self)))
        }
    }

    pub fn as_indirect(&'a self) -> Result<IndirectRef> {
        if let Object::Indirect(ind) = self {
            Ok(*ind)
        } else {
            Err(Error::Syntax(
                "Expected indirect",
                format!("got {:?}", self),
            ))
        }
    }
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

impl<'a> IntoIterator for &'a Object<'a> {
    type Item = &'a Object<'a>;
    type IntoIter = ObjectIter<'a>;

    fn into_iter(self) -> ObjectIter<'a> {
        if let Object::Array(array) = self {
            ObjectIter::Array {
                array: &array,
                index: 0,
            }
        } else if self == &Object::Null {
            ObjectIter::Single {
                object: self,
                consumed: true,
            }
        } else {
            ObjectIter::Single {
                object: self,
                consumed: false,
            }
        }
    }
}

pub enum ObjectIter<'a> {
    Array {
        array: &'a [Object<'a>],
        index: usize,
    },
    Single {
        object: &'a Object<'a>,
        consumed: bool,
    },
}

impl<'a> Iterator for ObjectIter<'a> {
    type Item = &'a Object<'a>;

    fn next(&mut self) -> Option<&'a Object<'a>> {
        match self {
            Self::Array { array, index } => {
                let res = array.get(*index);
                *index += 1;
                res
            }
            Self::Single { object, consumed } => {
                if *consumed {
                    None
                } else {
                    *consumed = true;
                    Some(object)
                }
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
    fn should_cast_bool() {
        let obj = Object::Boolean(true);
        assert_eq!(obj.as_bool().unwrap(), true);
    }

    #[test]
    fn should_cast_int() {
        let obj = Object::Integer(42);
        assert_eq!(obj.as_int().unwrap(), 42);
    }

    #[test]
    fn should_cast_real() {
        let obj = Object::Real(42.0);
        assert_eq!(obj.as_real().unwrap(), 42.0);
    }

    #[test]
    fn should_cast_string() {
        let obj = Object::String(Cow::Borrowed(b"Hello, world!"));
        assert_eq!(obj.as_string().unwrap(), Cow::Borrowed(b"Hello, world!"));
    }

    #[test]
    fn should_cast_name() {
        let obj = Object::Name(Cow::Borrowed(b"Hello, world!"));
        assert_eq!(obj.as_name().unwrap(), Cow::Borrowed(b"Hello, world!"));
    }

    #[test]
    fn should_cast_array() {
        let obj = Object::Array(vec![
            Object::Integer(1),
            Object::Integer(2),
            Object::Integer(3),
        ]);
        assert_eq!(obj.as_array().unwrap()[0].as_int().unwrap(), 1);
        assert_eq!(obj.as_array().unwrap()[1].as_int().unwrap(), 2);
        assert_eq!(obj.as_array().unwrap()[2].as_int().unwrap(), 3);
    }

    #[test]
    fn should_cast_dict() {
        let mut dict = HashMap::new();
        let key: Cow<[u8]> = Cow::Borrowed(b"Key");
        dict.insert(key.clone(), Object::Integer(1));

        let obj = Object::Dictionary(dict);
        assert_eq!(obj.as_dict().unwrap()[&key].as_int().unwrap(), 1);
    }

    #[test]
    fn should_cast_stream() {
        let mut dict = HashMap::new();
        let key: Cow<[u8]> = Cow::Borrowed(b"Key");
        dict.insert(key.clone(), Object::Integer(1));

        let obj = Object::Stream(
            Box::new(Object::Dictionary(dict)),
            Cow::Borrowed(b"Hello, world!"),
        );
        let (dict, stream) = obj.as_stream().unwrap();
        assert_eq!(dict[&key].as_int().unwrap(), 1);
        assert_eq!(stream, Cow::Borrowed(b"Hello, world!"));
    }

    #[test]
    fn should_cast_null() {
        let obj = Object::Null;
        obj.as_null().unwrap();
    }

    #[test]
    fn should_cast_indirect() {
        let obj = Object::Indirect(IndirectRef {
            number: 1,
            generation: 2,
        });
        assert_eq!(
            obj.as_indirect().unwrap(),
            IndirectRef {
                number: 1,
                generation: 2,
            }
        );
    }
}
