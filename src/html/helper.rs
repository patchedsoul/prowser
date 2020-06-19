use crate::data_storage;
use crate::dom;
use crate::html::Parser;
use crate::logic;

use std::collections::HashMap;

impl Parser {
    /// Parse a single element, including its open tag, contents, and closing tag (if present).
    pub fn parse_element(&mut self) -> Option<dom::Node> {
        // (Opening) tag.
        self.consume_char(); // <
        let tag_name = self.parse_tag_name();
        if tag_name.is_empty() {
            // maybe not correct behavior for `<=` an similar. But better than an endless loop
            return None;
        }

        let attributes = self.parse_attributes()?;

        if let Some('/') = self.next_char() {
            self.consume_char(); // /
        }
        if let Some('>') = self.next_char() {
            self.consume_char(); // >
        }

        let children;
        let array = [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
            "source", "track", "wbr",
        ];
        if array.contains(&&*tag_name.to_ascii_lowercase()) {
            children = Vec::new();

            if tag_name == "link" {
                if let Some(relationship) = attributes.get("rel") {
                    if relationship == "stylesheet" {
                        if let Some(raw_url) = attributes.get("href") {
                            let query;
                            let url = logic::absolute_path(&self.url, raw_url);
                            if let Some(media_query) = attributes.get("media") {
                                query = media_query.clone();
                            } else {
                                let _ = data_storage::download(&url).is_ok();
                                query = String::new();
                            }

                            self.style.push((url, Some(query)));
                        }
                    }
                }
            }
        } else {
            if tag_name == "style" {
                children = Vec::new();

                let mut value = String::new();
                while !self.starts_with("</style>") {
                    if let Some(c) = self.consume_char() {
                        value.push(c);
                    } else {
                        return None;
                    }
                }

                // https://html.spec.whatwg.org/multipage/semantics.html#update-a-style-block 4.
                if let Some(type_attribute) = attributes.get("type") {
                    if type_attribute.to_ascii_lowercase() != "text/css"
                        && !type_attribute.is_empty()
                    {
                        return None;
                    }
                }

                self.style.push((value, None));
            } else if tag_name == "script" {
                while !self.starts_with("</script>") {
                    self.consume_char();
                }
                return None;
            } else {
                // Contents.
                children = self.parse_nodes().0;
            }

            // in case closing tag is missing
            if !self.eof() {
                // Closing tag.
                if let Some('<') = self.next_char() {
                    self.consume_char(); // <
                }
                if let Some('/') = self.next_char() {
                    self.consume_char(); // /
                }
                self.parse_tag_name();
                self.parse_attributes();
                if let Some('/') = self.consume_char() {
                    self.consume_char(); // >
                }
            }
        }

        Some(dom::Node::elem(tag_name, attributes, children))
    }

    /// Parse a single name="value" pair.
    fn parse_attr(&mut self) -> (String, String) {
        let name = self.parse_tag_name();
        let mut value = if let Some('=') = self.next_char() {
            self.consume_char(); // =
            self.parse_attr_value()
        } else {
            String::new()
        };

        if name == "src" {
            value = logic::absolute_path(&self.url, &value);
        }

        (name, value)
    }

    /// Parse a (quoted) value.
    /// `target="_blank"` > "_blank" > _blank
    fn parse_attr_value(&mut self) -> String {
        let value;
        if let Some('"') | Some('\'') = self.next_char() {
            let open_quote = self.consume_char().unwrap(); // ' | "
            value = self.consume_while(|c| c != open_quote);
            self.consume_char(); // open_quote
        } else {
            value = self.parse_attribute_value();
        }

        value
    }

    /// Parse a list of name="value" pairs, separated by whitespace.
    fn parse_attributes(&mut self) -> Option<dom::AttrMap> {
        let mut attributes = HashMap::new();
        loop {
            self.consume_whitespace();
            match self.next_char() {
                Some('>') => break,
                Some('/') => {
                    self.consume_char(); // /
                    continue;
                }
                None => return None,
                _ => {}
            }

            let (name, value) = self.parse_attr();
            attributes.entry(name).or_insert(value);
        }
        Some(attributes)
    }

    /// Parse a sequence of sibling nodes.
    pub fn parse_nodes(&mut self) -> (Vec<dom::Node>, Vec<(String, Option<String>)>) {
        self.consume_whitespace();
        // <!doctype
        if self.starts_with("<!") {
            self.consume_while(|c| c != '>');
            self.consume_char(); // >
        }

        let mut nodes = Vec::new();
        loop {
            self.consume_whitespace();
            // <!-- comment
            if self.starts_with("<!--") {
                while !self.starts_with("-->") {
                    self.consume_char();
                }
                self.consume_char(); // -
                self.consume_char(); // -
                self.consume_char(); // >
                continue;
            }
            if self.eof() || self.starts_with("</") {
                break;
            }
            if let Some(node) = self.parse_node() {
                nodes.push(node);
            }
        }
        (nodes, self.style.clone())
    }
}

// FIXME: change mod name. maybe split a mod for each testet method.
#[cfg(test)]
mod parse_element {
    use super::*;

    #[test]
    fn style() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<style></style>"),
            url: String::new(),
            style: Vec::new(),
        };

        // FIXME: test style parsing correctly (whole returned Node)
        assert!(parser.parse_element().is_some());
    }

    #[test]
    fn type_attribute_invalid() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("<style type=' text/css '></style>"),
            url: String::new(),
            style: Vec::new(),
        };

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("<style type='text/css; charset=utf-8'></style>"),
            url: String::new(),
            style: Vec::new(),
        };

        assert!(parser1.parse_element().is_none());
        assert!(parser2.parse_element().is_none());
    }

    #[test]
    fn type_attribute_valid() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("<style type=''></style>"),
            url: String::new(),
            style: Vec::new(),
        };

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("<style type='text/CSS'></style>"),
            url: String::new(),
            style: Vec::new(),
        };

        assert!(parser1.parse_element().is_some());
        assert!(parser2.parse_element().is_some());
    }

    #[test]
    fn script() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<script>console.log('Test');</script>"),
            url: String::new(),
            style: Vec::new(),
        };

        assert!(parser.parse_element().is_none());
    }

    #[test]
    fn attribute() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("href='https://example.com'"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(
            parser.parse_attr(),
            (String::from("href"), String::from("https://example.com"))
        );
    }

    #[test]
    fn empty_attribute() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("href"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(parser.parse_attr(), (String::from("href"), String::new()));
    }

    #[test]
    fn attribute_value() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("'test'"),
            url: String::new(),
            style: Vec::new(),
        };

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("\"test\""),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(parser1.parse_attr_value(), String::from("test"));
        assert_eq!(parser2.parse_attr_value(), String::from("test"));
    }

    #[test]
    fn no_quote_attribute_value() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("test"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(parser.parse_attr_value(), String::from("test"));
    }

    /// example `<a href=https://git.sr.ht/~sircmpwn/sr.ht-docs>https://git.sr.ht/~sircmpwn/sr.ht-docs</a>`
    #[test]
    fn no_quote_attribute_value_link() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("https://git.sr.ht/~sircmpwn/sr.ht-docs"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(
            parser.parse_attr_value(),
            String::from("https://git.sr.ht/~sircmpwn/sr.ht-docs")
        );
    }

    #[test]
    fn attributes() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("href='https://example.com' target='_blank'>"),
            url: String::new(),
            style: Vec::new(),
        };

        let mut result = HashMap::new();
        result.insert(String::from("target"), String::from("_blank"));
        result.insert(String::from("href"), String::from("https://example.com"));

        assert_eq!(parser.parse_attributes(), Some(result));
    }

    #[test]
    fn duplicate_attributes() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("href='https://example.com' href='https://test.com'>"),
            url: String::new(),
            style: Vec::new(),
        };

        let mut result = HashMap::new();
        result.insert(String::from("href"), String::from("https://example.com"));

        assert_eq!(parser.parse_attributes(), Some(result));
    }

    #[test]
    fn nodes_doctype() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<!DOCTYPE html>"),
            url: String::new(),
            style: Vec::new(),
        };

        let result = parser.parse_nodes();

        assert_eq!(parser.pos, 15);
        assert!(result.0.is_empty());
    }

    // FIXME: not testing comment removing part.
    // parse_node seems to eat it and return `None`
    #[test]
    fn nodes_comment() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<!-- comment -->"),
            url: String::new(),
            style: Vec::new(),
        };

        let result = parser.parse_nodes();

        dbg!(&result);
        dbg!(&parser);

        assert!(result.0.is_empty());
        assert_eq!(parser.pos, 16);
    }
}
