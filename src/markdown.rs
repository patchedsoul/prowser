//! Markdown to HTML parser

/*
Current status: WIP, proof of concept

Goal:
- minimum viable product
- not 100% of the markdown spec
- simple but functional
*/

#[derive(Debug)]
struct Parser {
    pos: usize,
    input: String,
    url: String,
}

impl Parser {
    /// Read the current character without consuming it.
    fn next_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Do the next characters start with the given string?
    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    /// Return true if all input is consumed.
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
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

    /// Consume and discard zero or more whitespace characters.
    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    pub fn parse(&mut self) -> String {
        let mut document = String::new();

        loop {
            self.consume_whitespace();

            if self.eof() {
                break;
            }
            if self.starts_with("# ") {
                self.consume_char(); // #
                self.consume_char();

                let heading = self.consume_while(|c| c != '\n');

                document.push_str("<h1>");
                document.push_str(&heading);
                document.push_str("</h1>");
            } else if self.starts_with("## ") {
                self.consume_char(); // #
                self.consume_char(); // #
                self.consume_char();

                let heading = self.consume_while(|c| c != '\n');

                document.push_str("<h2>");
                document.push_str(&heading);
                document.push_str("</h2>");
            } else if self.starts_with("### ") {
                self.consume_char(); // #
                self.consume_char(); // #
                self.consume_char(); // #
                self.consume_char();

                let heading = self.consume_while(|c| c != '\n');

                document.push_str("<h3>");
                document.push_str(&heading);
                document.push_str("</h3>");
            } else if self.starts_with(">") {
                self.consume_char(); // #

                let quote = self.consume_while(|c| c != '\n');

                document.push_str("<q>");
                document.push_str(&quote);
                document.push_str("</q>");
            } else if self.starts_with("[") {
                // FIXME: links can occure in every text
                self.consume_char(); // [
                let link_text = self.consume_while(|c| c != ']');
                self.consume_char(); // ]

                self.consume_char(); // (
                let link_url = self.consume_while(|c| c != ')');
                self.consume_char(); // )

                document.push_str(&format!("<a href=\"{}\">", link_url));
                document.push_str(&link_text);
                document.push_str("</a>");
            } else if self.starts_with("```") {
                self.consume_char(); // `
                self.consume_char(); // `
                self.consume_char(); // `

                // FIXME: parse till ``` and not just a singular `
                let code = self.consume_while(|c| c != '`');
                self.consume_char(); // `
                self.consume_char(); // `
                self.consume_char(); // `

                document.push_str("<code>");
                document.push_str(&code);
                document.push_str("</code>");
            } else if self.starts_with("`") {
                self.consume_char(); // `
                let code = self.consume_while(|c| c != '`');
                self.consume_char(); // `

                document.push_str("<code>");
                document.push_str(&code);
                document.push_str("</code>");
            } else if self.starts_with("---") {
                self.consume_while(|c| c != '\n');

                document.push_str("<hr>");
            } else {
                let paragraph = self.consume_while(|c| c != '\n');

                document.push_str("<p>");
                document.push_str(&paragraph);
                document.push_str("</p>");
            }

            self.consume_char();
        }

        document
    }
}

/// Parse an Markdown document and return the parsed HTML Source.
pub fn parse(source: String, url: String) -> String {
    Parser {
        pos: 0,
        input: source,
        url,
    }
    .parse()
}

#[cfg(test)]
mod parser {
    use super::*;

    #[test]
    fn parse_heading() {
        let mut parser = Parser {
            pos: 0,
            input: String::from(
                "# Headin 1

            ## heading 2

            ### heading 3",
            ),
            url: String::new(),
        };

        assert_eq!(
            parser.parse(),
            String::from("<h1>Headin 1</h1><h2>heading 2</h2><h3>heading 3</h3>")
        );
    }

    #[test]
    fn parse_paragraph() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("Paragraph"),
            url: String::new(),
        };

        assert_eq!(parser.parse(), String::from("<p>Paragraph</p>"));
    }

    #[test]
    fn parse_link() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("[some link](https://example.com)"),
            url: String::new(),
        };

        assert_eq!(
            parser.parse(),
            String::from("<a href=\"https://example.com\">some link</a>")
        );
    }

    #[test]
    fn parse_qoute() {
        let mut parser = Parser {
            pos: 0,
            input: String::from(">Quote"),
            url: String::new(),
        };

        assert_eq!(parser.parse(), String::from("<q>Quote</q>"));
    }

    #[test]
    fn parse_hr() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("---"),
            url: String::new(),
        };

        assert_eq!(parser.parse(), String::from("<hr>"));
    }

    #[test]
    fn parse_inline_code() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("`let x;`"),
            url: String::new(),
        };

        assert_eq!(parser.parse(), String::from("<code>let x;</code>"));
    }

    #[test]
    fn parse_multiline_code() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("```let x;\ndbg!();```"),
            url: String::new(),
        };

        assert_eq!(parser.parse(), String::from("<code>let x;\ndbg!();</code>"));
    }
}
