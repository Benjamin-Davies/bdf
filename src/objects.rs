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
}
