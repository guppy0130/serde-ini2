use pest_derive::Parser;

// use crate::Error;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct IniParser;

#[cfg(test)]
mod tests {
    use pest::{iterators::Pair, Parser};

    use super::*;

    #[test]
    fn test_parser() {
        let input = "[Test]
key=value
";
        let parsed = IniParser::parse(Rule::file, input).unwrap().flatten();

        // there is only one k_v_pair
        assert_eq!(
            parsed
                .clone()
                .filter(|pair| pair.as_rule() == Rule::k_v_pair)
                .collect::<Vec<Pair<Rule>>>()
                .len(),
            1
        );
    }

    #[test]
    fn test_two_map() {
        let input = "key=value\nkey2=value2";
        let parsed = IniParser::parse(Rule::file, input).unwrap().flatten();
        assert_eq!(
            parsed
                .into_iter()
                .filter(|pair| pair.as_rule() == Rule::k_v_pair)
                .collect::<Vec<Pair<Rule>>>()
                .len(),
            2
        )
    }
}
