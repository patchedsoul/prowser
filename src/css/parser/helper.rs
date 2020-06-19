use crate::css::parser::Parser;
use crate::css::{check_color_keyword, valid_identifier_char, valid_unit_char, Color, Unit, Value};
use crate::logic;

impl Parser {
    /// Methods for parsing one or multiple values
    pub fn parse_values(&mut self) -> Option<(Vec<Value>, bool)> {
        let mut values = Vec::new();
        let mut important = false;

        loop {
            if let Some(value) = self.parse_value() {
                values.push(value);
            } else {
                return None;
            }
            self.consume_blank();

            if let Some(';') = self.next_char() {
                self.consume_char(); // ;
                break;
            } else if let Some('}') = self.next_char() {
                break;
            } else if let Some(',') | Some('/') = self.next_char() {
                // TODO: handle list and `/` correctly. see media_query > aspect ratio
                self.consume_char();
                self.consume_blank();
            } else if let Some('!') = self.next_char() {
                self.consume_char();
                if self.parse_identifier().to_ascii_lowercase() == "important" {
                    important = true;
                    break;
                } else {
                    return None;
                }
            } else if self.eof() {
                break;
            }
        }
        Some((values, important))
    }

    /// Methods for parsing a value (like `5px` or `rgba()`)
    pub fn parse_value(&mut self) -> Option<Value> {
        match self.next_char() {
            Some('-') => {
                // value could be `-4px` or `-apple-system`.
                self.pos += 1;
                let v = if let Some('0'..='9') | Some('.') = self.next_char() {
                    self.parse_length()
                } else {
                    let open_quote = self.consume_char().unwrap(); // ' | "
                    let value = self.consume_while(|c| c != open_quote);
                    self.consume_char(); // open_quote
                    Some(Value::Str(value))
                };

                self.pos -= 1;
                v
            }
            Some('0'..='9') | Some('.') => self.parse_length(),
            Some('#') => self.parse_hex_color(),
            Some('"') | Some('\'') => {
                let open_quote = self.consume_char().unwrap(); // ' | "
                let value = self.consume_while(|c| c != open_quote);
                self.consume_char(); // open_quote

                // `"(" attr(title) ")"` is valid
                self.consume_blank();
                if self.starts_with("attr") {
                    self.consume_while(|c| c != ';' && c != '}');
                    return None;
                }

                Some(Value::Str(value))
            }
            _ => {
                let keyword = self.parse_identifier().to_ascii_lowercase();
                if keyword.is_empty() {
                    return None;
                }

                if let Some(value) = check_color_keyword(&keyword) {
                    return Some(value);
                }

                // TODO: parse remaining functions
                //https://www.w3schools.com/csSref/css_functions.asp
                match &*keyword {
                    "attr"
                    | "calc"
                    | "cubic-bezier"
                    | "repeat"
                    | "repeating-linear-gradient"
                    | "repeating-radial-gradient"
                    | "scale"
                    | "rotateY" => {
                        // TODO: split out in seperate arm
                        self.consume_while(|c| c != ';' && c != '}');
                        None
                    }
                    "hsl" => {
                        let (r, g, b) = self.parse_hsl();
                        self.consume_char(); // )

                        Some(Value::Color(Color { r, g, b, a: 255 }))
                    }
                    "hsla" => {
                        let (r, g, b) = self.parse_hsl();
                        self.consume_char(); // ,
                        self.consume_blank();

                        let a = self.consume_while(|c| match c {
                            '0'..='9' | '.' => true,
                            _ => false,
                        });
                        self.consume_blank();
                        self.consume_char(); // )

                        let a = (a.parse::<f32>().unwrap() * 255.0) as u8;

                        Some(Value::Color(Color { r, g, b, a }))
                    }
                    "rgb" => {
                        let (r, g, b) = self.parse_rgb();
                        self.consume_char(); // )

                        Some(Value::Color(Color { r, g, b, a: 255 }))
                    }
                    "rgba" => {
                        let (r, g, b) = self.parse_rgb();
                        self.consume_char(); // ,
                        self.consume_blank();

                        let a = self.consume_while(|c| match c {
                            '0'..='9' | '.' => true,
                            _ => false,
                        });
                        self.consume_blank();
                        self.consume_char(); // )

                        let a = (a.parse::<f32>().unwrap() * 255.0) as u8;

                        Some(Value::Color(Color { r, g, b, a }))
                    }
                    "linear-gradient" | "radial-gradient" => {
                        self.consume_char(); // (
                        self.consume_blank();

                        let mut colors = Vec::new();

                        loop {
                            if let Some(Value::Color(color)) = self.parse_value() {
                                colors.push(color);
                            }
                            self.consume_blank();
                            if let Some(',') = self.next_char() {
                                self.consume_char(); // ,
                                self.consume_blank();
                            } else if let Some(')') = self.next_char() {
                                break;
                            }
                        }
                        self.consume_char(); // )

                        Some(Value::Gradient(0, colors))
                    }
                    "url" => {
                        self.consume_char(); // (
                        let url;
                        if let Some('"') | Some('\'') = self.next_char() {
                            let open_quote = self.consume_char().unwrap(); // ' | "
                            url = self.consume_while(|c| c != open_quote);
                            self.consume_char(); // open_quote
                        } else {
                            url = self.consume_while(|c| c != ')');
                        }
                        self.consume_char(); // )

                        Some(Value::Url(logic::absolute_path(&self.url, &url)))
                    }
                    "var" => {
                        // FIXME: reads backup value, but not actual variable
                        self.consume_char(); // (
                        self.consume_blank();
                        self.parse_identifier();
                        self.consume_blank();

                        let value = if let Some(',') = self.next_char() {
                            self.consume_char(); // ,
                            self.consume_blank();
                            self.parse_value()
                        } else {
                            None
                        };

                        self.consume_blank();
                        self.consume_char(); // )
                        value
                    }
                    "env" => {
                        self.consume_char(); // (
                        self.consume_blank();
                        let identifier = self.parse_identifier().to_ascii_lowercase();
                        self.consume_blank();

                        let value = if identifier == "safe-area-inset-top"
                            || identifier == "safe-area-inset-right"
                            || identifier == "safe-area-inset-bottom"
                            || identifier == "safe-area-inset-left"
                        {
                            // For rectangular viewports, like your average laptop monitor, their value is equal to zero.
                            Some(Value::Length(0.0, Unit::Zero))
                        } else if let Some(',') = self.next_char() {
                            // A fallback value in case the environment variable is not available.
                            self.consume_char(); // ,
                            self.consume_blank();
                            self.parse_value()
                        } else {
                            None
                        };

                        self.consume_blank();
                        self.consume_char(); // )
                        value
                    }
                    _ => Some(Value::Keyword(keyword)),
                }
            }
        }
    }

    fn parse_length(&mut self) -> Option<Value> {
        self.parse_float()
            .map(|float| Value::Length(float, self.parse_unit()))
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

    /// Parse a property name or keyword.
    /// `'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_'`
    pub fn parse_identifier(&mut self) -> String {
        self.consume_while(valid_identifier_char)
    }

    /// Parse a unit.
    /// `'a'..='z' | 'A'..='Z' | '%'`
    fn parse_valid_unit(&mut self) -> String {
        self.consume_while(valid_unit_char)
    }

    /// Consume characters until `test` returns false.
    pub fn consume_while<F>(&mut self, test: F) -> String
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
    pub fn consume_char(&mut self) -> Option<char> {
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
    pub fn next_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Do the next characters start with the given string?
    pub fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    /// Return true if all input is consumed.
    pub fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// consume whitespaces and `/* comments */`
    pub fn consume_blank(&mut self) {
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

#[cfg(test)]
mod parse_element {
    use super::*;

    #[test]
    fn blank() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("    	       /*  ad as d */    	a    "),
            url: String::new(),
        };

        parser.consume_blank();
        // consumes till `a`
        assert_eq!(parser.pos, 31);
    }

    #[test]
    fn unit_none() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("hallo"),
            url: String::new(),
        };

        assert_eq!(parser.parse_unit(), Unit::Zero);
    }

    #[test]
    fn unit_px() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("px"),
            url: String::new(),
        };

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("rEM"),
            url: String::new(),
        };

        assert_eq!(parser1.parse_unit(), Unit::Px);
        assert_eq!(parser2.parse_unit(), Unit::Rem);
    }
}
