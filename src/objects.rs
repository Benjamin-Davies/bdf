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

impl<'a> Object<'a> {
    pub fn index_array(&self, index: usize) -> &Object<'a> {
        if let Object::Array(array) = self {
            &array.get(index).unwrap_or(&Object::Null)
        } else {
            &Object::Null
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
