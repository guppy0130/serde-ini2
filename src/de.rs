use std::collections::VecDeque;

use pest::iterators::{Pair, Pairs};
use pest::Parser;
use serde::de::{self, DeserializeSeed, Visitor};
use serde::{forward_to_deserialize_any, Deserialize};

use crate::error::{Error, Result};
use crate::parser::{IniParser, Rule};

pub struct Deserializer<'de> {
    pairs: Vec<Pair<'de, Rule>>,
}

impl<'de> Deserializer<'de> {
    // TODO: should this be public?
    fn from_str(input: &'de str) -> Result<Self> {
        let pairs = IniParser::parse(Rule::file, input).map_err(|e| Error::Parse(Box::new(e)))?;
        Ok(Deserializer::from_pairs(pairs))
    }
    fn from_pair(pair: Pair<'de, Rule>) -> Self {
        Deserializer { pairs: vec![pair] }
    }
    fn from_pairs(pairs: Pairs<'de, Rule>) -> Self {
        Deserializer {
            pairs: pairs.collect(),
        }
    }
}

pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s)?;
    T::deserialize(&mut deserializer)
}

/// return true for pairs that are sections and whose header (e.g., all the
/// words before the k/v pairs start), when all joined together without a
/// separator, matches `match_header`
fn pair_is_section_with_header(pair: &Pair<Rule>, match_header: &str) -> bool {
    pair.as_rule() == Rule::section
        && pair
            .clone() // TODO: expensive?
            .into_inner()
            .filter(|pair| pair.as_rule() == Rule::word)
            .fold(String::new(), |acc, x| acc + x.as_str())
            == match_header
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let pair = self.pairs.first().unwrap();
        match pair.as_rule() {
            Rule::k_v_pair => visitor.visit_map(Map::new(pair.to_owned())),
            Rule::word => visitor.visit_string(pair.as_str().to_owned()),
            _ => {
                dbg!(pair);
                Err(Error::UnsupportedType)
            }
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // TODO: handle empty values
        let pairs = self
            .pairs
            .iter_mut()
            .take_while(|pair| pair.as_rule() == Rule::k_v_pair) // get all the naked kv pairs
            .flat_map(|pair| pair.to_owned().into_inner()) // into_inner to get the individual words
            .take_while(|pair| pair.as_rule() == Rule::word); // ensure they're actually words

        visitor.visit_map(Map::new_via_iter(pairs))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // search for a section whose name matches and hand off all the kv pairs in it to the mapper
        dbg!(name);
        let pairs = self.pairs.to_owned();
        if let Some(matching_section) = pairs
            .iter()
            .find(|pair| pair_is_section_with_header(pair, name))
        {
            let mut iter = matching_section.clone().into_inner();

            // skip the header
            while let Some(item) = iter.peek() {
                if item.as_rule() == Rule::word {
                    iter.next();
                } else {
                    break;
                }
            }

            self.pairs = iter.collect();
        } else {
            // TODO: incomplete; corresponding test marked as incomplete as well
            todo!("Incomplete");
            // maybe you're a container type and the sections are the fields
            #[allow(unreachable_code)]
            self.pairs = pairs
                .iter()
                .filter(|pair| {
                    fields
                        .iter()
                        .any(|field| pair_is_section_with_header(pair, field))
                })
                .map(|item| item.to_owned())
                .collect();
        }
        self.deserialize_map(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct enum identifier ignored_any
    }
}

struct Map<'de> {
    pairs: VecDeque<Pair<'de, Rule>>,
}

impl<'de> Map<'de> {
    /// hand this a section without a header...?
    pub fn new(pair: Pair<'de, Rule>) -> Self {
        Self {
            pairs: pair.into_inner().collect(),
        }
    }

    /// assumes already flattened k/v pairs are handed to us
    pub fn new_via_iter(pairs: impl Iterator<Item = Pair<'de, Rule>>) -> Self {
        Self {
            pairs: pairs.collect(),
        }
    }
}

impl<'de> de::MapAccess<'de> for Map<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if let Some(pair) = self.pairs.pop_front() {
            seed.deserialize(&mut Deserializer::from_pair(pair))
                .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut Deserializer::from_pair(
            self.pairs.pop_front().unwrap(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_map() {
        let input = "key=value\nkey2=value2";
        let mut expected: HashMap<String, String> = HashMap::new();
        expected.insert("key".to_owned(), "value".to_owned());
        expected.insert("key2".to_owned(), "value2".to_owned());
        assert_eq!(
            expected,
            from_str::<HashMap<String, String>>(input).unwrap(),
        )
    }

    #[test]
    /// duplicate keys last wins
    fn test_map_duplicate_key() {
        let input = "key=value\nkey=value2";
        let mut expected: HashMap<String, String> = HashMap::new();
        expected.insert("key".to_owned(), "value2".to_owned());
        assert_eq!(
            expected,
            from_str::<HashMap<String, String>>(input).unwrap(),
        )
    }

    #[test]
    fn test_struct() {
        let input = "[Test]
key=value
key2=value2
";
        #[derive(Deserialize, Debug, PartialEq, Eq)]
        struct Test {
            key: String,
            key2: String,
        }
        let expected = Test {
            key: "value".to_owned(),
            key2: "value2".to_owned(),
        };
        assert_eq!(from_str::<Test>(input).unwrap(), expected)
    }

    #[test]
    #[ignore = "incomplete"]
    fn test_extra_content_not_serialized() {
        let input = "[job_builder]
ignore_cache=True

[jenkins]
user=jenkins
";

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct JobBuilder {
            ignore_cache: String,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Jenkins {
            user: String,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        struct Container {
            job_builder: JobBuilder,
            jenkins: Jenkins,
        }

        dbg!(from_str::<Container>(input).unwrap());
        panic!("");
    }
}
