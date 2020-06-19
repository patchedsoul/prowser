use crate::css::media_query::{matches, MediaFeature, MediaQuery};
use crate::css::{self, Unit, Value};

pub struct Parser {
    pub input: String,
    pub pos: usize,
}

impl Parser {
    /// Parses queries and checks if **one** matches
    pub fn matches(&mut self, dimensions: (u32, u32)) -> bool {
        self.consume_blank();

        // `@media { … }` = `@media all { … }`
        if self.eof() {
            return true;
        }
        let queries = self.parse_queries();

        queries.iter().any(|query| matches(query, dimensions))
    }

    /// Parses queries `screen, print and (color)`
    fn parse_queries(&mut self) -> Vec<MediaQuery> {
        let mut queries = Vec::with_capacity(1);

        while self.next_char().is_some() {
            queries.push(self.parse_query());

            self.consume_char(); // ,
            self.consume_blank();
        }

        queries
    }

    /// Parses single query `not print and (color)`
    // TODO: hande errors. What if no feature follows `and` for example
    fn parse_query(&mut self) -> MediaQuery {
        let mut query = MediaQuery {
            media_type: String::new(),
            media_features: Vec::new(),
            not: false,
        };

        if self.starts_with("not") {
            query.not = true;
            self.parse_keyword(); // not
            self.consume_blank();
        } else if self.starts_with("only") {
            self.parse_keyword();
            self.consume_blank();
        }

        // if no feature follows, it must be a media type
        if !self.starts_with("(") {
            query.media_type = self.parse_keyword().to_ascii_lowercase();
            self.consume_blank();

            if self.starts_with("and") {
                self.parse_keyword();
                self.consume_blank();
            }
        }

        if !self.eof() {
            loop {
                if let Some(feature) = self.parse_feature() {
                    self.consume_blank();
                    if self.starts_with("and") {
                        self.parse_keyword();
                        query.media_features.push((feature, '&'));
                    } else if self.starts_with("or") {
                        self.parse_keyword();
                        query.media_features.push((feature, '|'));
                    } else {
                        query.media_features.push((feature, '-'));
                    }
                } else {
                    self.consume_blank();
                    break;
                }

                self.consume_blank();
            }
        }

        query
    }

    /// Parses a feature, `(min-width: 30em)`
    fn parse_feature(&mut self) -> Option<MediaFeature> {
        match self.next_char() {
            Some('(') => {
                self.consume_char(); // (
                self.consume_blank();

                let name = self.parse_feature_keyword().to_ascii_lowercase();
                let mut value = None;

                self.consume_blank();

                match self.next_char() {
                    Some(':') => {
                        self.consume_char(); // :
                        self.consume_blank();

                        value = self.parse_value();
                        self.consume_blank();
                    }
                    Some(')') => {}
                    _ => panic!("unallowed char"),
                }

                let condition = if let Some(value) = value {
                    Some(MediaFeature::Declaration(
                        css::Declaration {
                            name,
                            value,
                            important: false,
                        },
                        false,
                    ))
                } else {
                    Some(MediaFeature::Name(name, false))
                };

                self.consume_char(); // )

                condition
            }
            None | Some(',') => None,
            Some(c) => panic!(
                "unallowed character {} (pos: {}, self: {})",
                c, self.pos, self.input
            ),
        }
    }

    /// Parse value `30em`, `3 / 2`
    fn parse_value(&mut self) -> Option<Value> {
        match self.next_char() {
            Some('0'..='9') | Some('-') | Some('.') => {
                let float = self.parse_float().unwrap();

                if self.starts_with("dpi") {
                    self.parse_keyword(); // dpi
                    return Some(Value::Number(float as u32));
                }

                self.consume_blank();
                match self.next_char() {
                    Some('a'..='z') | Some('A'..='Z') => {
                        Some(Value::Length(float, self.parse_unit()))
                    }
                    Some('/') => {
                        // ratio
                        self.consume_char(); // /
                        self.consume_blank();

                        // FIXME: unclean to parse float and then convert it
                        let float2 = self.parse_float().unwrap() as u32;
                        Some(Value::Ratio(float as u32, float2))
                    }
                    Some(')') => Some(Value::Number(float as u32)),
                    _ => None,
                }
            }
            _ => {
                let keyword = self.parse_feature_keyword().to_ascii_lowercase();
                if keyword.is_empty() {
                    return None;
                }
                Some(Value::Keyword(keyword))
            }
        }
    }

    fn parse_float(&mut self) -> Option<f32> {
        let s = self.consume_while(|c| match c {
            '0'..='9' | '.' | '-' => true,
            _ => false,
        });
        s.parse().ok()
    }

    fn parse_unit(&mut self) -> Unit {
        match &*self.parse_valid_unit().to_ascii_lowercase() {
            "%" => Unit::Percentage,
            "ch" => Unit::Ch,
            "cm" => Unit::Cm,
            "em" => Unit::Em,
            "ex" => Unit::Ex,
            "in" => Unit::In,
            "mm" => Unit::Mm,
            "pc" => Unit::Pc,
            "pt" => Unit::Pt,
            "px" => Unit::Px,
            "q" => Unit::Q,
            "rem" => Unit::Rem,
            "vh" => Unit::Vh,
            "vmax" => Unit::Vmax,
            "vmin" => Unit::Vmin,
            "vw" => Unit::Vw,
            _ => Unit::Zero,
        }
    }

    /// Parse a keyword.
    /// `'a'..='z' | 'A'..='Z'`
    fn parse_keyword(&mut self) -> String {
        self.consume_while(valid_keyword)
    }

    /// Parse a feature keyword. `min-width`
    /// `'a'..='z' | 'A'..='Z' | '-' | '0'..='9'`
    fn parse_feature_keyword(&mut self) -> String {
        self.consume_while(valid_feature)
    }

    /// Parse a unit.
    /// `'a'..='z' | 'A'..='Z' | '%'`
    fn parse_valid_unit(&mut self) -> String {
        self.consume_while(valid_unit_char)
    }

    /// Consume characters until `test` returns false.
    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && test(self.next_char().unwrap()) {
            result.push(self.consume_char().unwrap());
        }
        result
    }

    /// Return the current character, and advance `self.pos` to the next character.
    fn consume_char(&mut self) -> Option<char> {
        let mut iter = self.input[self.pos..].char_indices();
        if let Some((_, cur_char)) = iter.next() {
            let (next_pos, _) = iter.next().unwrap_or((1, ' '));
            self.pos += next_pos;
            Some(cur_char)
        } else {
            None
        }
    }

    /// Read the current character without consuming it.
    fn next_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Do the next characters start with the given (lowercase) string?
    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].to_ascii_lowercase().starts_with(s)
    }

    /// Return true if all input is consumed.
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// consume whitespaces and `/* comments */`
    fn consume_blank(&mut self) {
        loop {
            // Consume and discard zero or more whitespace characters.
            self.consume_while(char::is_whitespace);
            // consumes a `/* comment */` if present
            if self.starts_with("/*") {
                while !self.starts_with("*/") {
                    self.consume_char();
                }
                self.consume_char(); // *
                self.consume_char(); // /
            } else {
                break;
            }
        }
    }
}

/// `'a'..='z' | 'A'..='Z'`
fn valid_keyword(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' => true,
        _ => false,
    }
}

/// `'a'..='z' | 'A'..='Z' | '-' | '0'..='9'`
fn valid_feature(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' | '-' | '0'..='9' => true,
        _ => false,
    }
}

/// `'a'..='z' | 'A'..='Z' | '%'`
fn valid_unit_char(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' | '%' => true,
        _ => false,
    }
}

#[cfg(test)]
mod parse {
    use super::*;

    #[test]
    fn unit_percentage() {
        let mut p = Parser {
            pos: 0,
            input: String::from("%"),
        };

        assert_eq!(p.parse_unit(), Unit::Percentage);
    }

    #[test]
    fn unit_undefined() {
        let mut p = Parser {
            pos: 0,
            input: String::from("sdfsdf"),
        };

        assert_eq!(p.parse_unit(), Unit::Zero);
    }

    #[test]
    fn parse_ratio() {
        let mut p = Parser {
            pos: 0,
            input: String::from("8/5"),
        };

        assert_eq!(p.parse_value(), Some(Value::Ratio(8, 5)));
    }

    #[test]
    fn parse_dpi() {
        let mut p = Parser {
            pos: 0,
            input: String::from("153dpi"),
        };

        assert_eq!(p.parse_value(), Some(Value::Number(153)));
    }

    /// Parse a feature without any values
    #[test]
    fn parse_feature_only() {
        let mut p = Parser {
            pos: 0,
            input: String::from("(color)"),
        };

        let success;

        if let Some(MediaFeature::Name(name, false)) = p.parse_feature() {
            success = name == "color";
        } else {
            success = false;
        }

        assert!(success, "Name feature to be parsed");
    }

    #[test]
    fn parse_not() {
        let mut p = Parser {
            pos: 0,
            input: String::from("not print"),
        };

        let query = p.parse_query();

        assert_eq!(query.not, true);
        assert_eq!(query.media_type, String::from("print"));
    }

    #[test]
    fn parse_only() {
        let mut p = Parser {
            pos: 0,
            input: String::from("only aural"),
        };

        let query = p.parse_query();

        assert_eq!(query.not, false);
        assert_eq!(query.media_type, String::from("aural"));
    }

    /// CSS style sheets are generally case-insensitive, and this is also the case for media queries.
    #[test]
    fn parse_case_insensitive() {
        let mut p = Parser {
            pos: 0,
            input: String::from("nOt priNT And (coLOr)"),
        };

        let query = p.parse_query();

        assert_eq!(query.not, true);
        assert_eq!(query.media_type, String::from("print"));

        let success;

        if let MediaFeature::Name(name, false) = &query.media_features[0].0 {
            success = name == "color";
        } else {
            success = false;
        }

        assert!(success, "Name feature to be parsed");
    }
}
