use crate::data_storage;
use crate::dom;
use crate::html::Parser;
use crate::logic;

use std::collections::HashMap;

impl Parser {
    /// Parse a single element, including its open tag, contents, and closing tag (if present).
    /// `<a href="">link</a>`
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
                Some(_) => {}
            }

            let (name, value) = self.parse_attr();
            /* "Authors can include data for inline client-side scripts or server-side site-wide scripts to process using the data-*="" attributes.
            These are guaranteed to never be touched by browsers, and allow scripts to include data on HTML elements that scripts can then look for and process."
            Therefore just throw it away */
            if !name.starts_with("data-") {
                attributes.entry(name).or_insert(value);
            }
        }
        Some(attributes)
    }

    /// Parse a sequence of sibling nodes.
    pub fn parse_nodes(&mut self) -> (Vec<dom::Node>, Vec<(String, Option<String>)>) {
        self.consume_whitespace();
        // <!doctype and <![CDATA
        if self.starts_with("<!") {
            self.consume_while(|c| c != '>');
            self.consume_char(); // >
        }

        let mut nodes = Vec::new();
        loop {
            self.consume_whitespace();
            // <!-- comment
            // do not create Comment nodes
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

    /// ignore script elements
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

    /// the data-*="" attributes is guaranteed to never be touched by browsers
    #[test]
    fn data_attribute() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("data-src='https://example.com' target='_blank'>"),
            url: String::new(),
            style: Vec::new(),
        };

        let mut result = HashMap::new();
        result.insert(String::from("target"), String::from("_blank"));

        assert_eq!(parser.parse_attributes(), Some(result));
    }

    /// first definition is dominant "The parser ignores all such duplicate occurrences of the attribute."
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

    /// ignore doctype (as it isn't used atm)
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

    /// ignore cdata
    #[test]
    fn cdata() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<![CDATA[some stuff]]>"),
            url: String::new(),
            style: Vec::new(),
        };

        let result = parser.parse_nodes();

        assert!(result.0.is_empty());
        assert_eq!(parser.pos, 22);
    }

    /* TODO: https://html.spec.whatwg.org/#parse-errors

    - character-reference-outside-unicode-range
    - control-character-in-input-stream
    - invalid-first-character-of-tag-name

    */

    /// https://html.spec.whatwg.org/#parse-error-abrupt-doctype-public-identifier abrupt-doctype-system-identifier
    #[test]
    fn abrupt_doctype_public_identifier() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<!DOCTYPE html PUBLIC \"foo>"),
            url: String::new(),
            style: Vec::new(),
        };

        let result = parser.parse_nodes();

        assert_eq!(parser.pos, 27);
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

        assert!(result.0.is_empty());
        assert_eq!(parser.pos, 16);
    }

    // FIXME: not really tested atm "Attributes in end tags are completely ignored and do not make their way into the DOM."
    #[test]
    fn end_tag_with_attributes() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<div id=foo></div class=bar>"),
            url: String::new(),
            style: Vec::new(),
        };

        assert!(parser.parse_element().is_some());
        assert_eq!(parser.pos, 28);
    }

    #[test]
    fn end_tag_with_trailing_solidus() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<div></div/>"),
            url: String::new(),
            style: Vec::new(),
        };

        assert!(parser.parse_element().is_some());
        assert_eq!(parser.pos, 12);
    }

    /*
    FIXME: text part not handled atm
    "This error occurs if the parser encounters the end of the input stream where a tag name is expected. In this case the parser treats the beginning of a start tag (i.e., <) or an end tag (i.e., </) as text content."
    */
    #[test]
    fn eof_before_tag_name() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<"),
            url: String::new(),
            style: Vec::new(),
        };

        assert!(parser.parse_element().is_none());
        assert_eq!(parser.pos, 1);
    }

    #[test]
    fn eof_in_cdata_doctype() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<![CDATA"),
            url: String::new(),
            style: Vec::new(),
        };

        let result = parser.parse_nodes();

        assert!(result.0.is_empty());
        assert_eq!(parser.pos, 8);
    }

    #[test]
    fn eof_in_tag() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<div id="),
            url: String::new(),
            style: Vec::new(),
        };

        assert!(parser.parse_element().is_none());
        assert_eq!(parser.pos, 8);
    }

    #[test]
    fn incorrectly_closed_comment() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<!-- comment --!>"),
            url: String::new(),
            style: Vec::new(),
        };

        let result = parser.parse_nodes();

        assert!(result.0.is_empty());
        assert_eq!(parser.pos, 17);
    }

    #[test]
    fn abrupt_closing_of_empty_comment() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("<!-->"),
            url: String::new(),
            style: Vec::new(),
        };

        let result1 = parser1.parse_nodes();

        assert!(result1.0.is_empty());
        assert_eq!(parser1.pos, 5);

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("<!--->"),
            url: String::new(),
            style: Vec::new(),
        };

        let result2 = parser2.parse_nodes();

        assert!(result2.0.is_empty());
        assert_eq!(parser2.pos, 6);
    }

    #[test]
    fn incorrectly_opened_comment() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("<! treated as comment >"),
            url: String::new(),
            style: Vec::new(),
        };

        let result = parser.parse_nodes();

        assert!(result.0.is_empty());
        assert_eq!(parser.pos, 23);
    }

    #[test]
    fn missing_attribute_value() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("id=>"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(parser.parse_attr(), (String::from("id"), String::new()));
    }

    #[test]
    fn unexpected_character_in_attribute_name() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("foo<div"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(
            parser1.parse_attr(),
            (String::from("foo<div"), String::new())
        );

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("id'bar'"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(
            parser2.parse_attr(),
            (String::from("id'bar'"), String::new())
        );
    }

    #[test]
    fn unexpected_character_in_unquoted_attribute_value() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("foo=b'ar'"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(
            parser.parse_attr(),
            (String::from("foo"), String::from("b'ar'"))
        );
    }

    // FIXME: Due to a forgotten attribute name the parser treats this markup as a div element with
    // two attributes: a "foo" attribute with a "bar" value and a "="baz"" attribute with an empty value.
    #[ignore]
    #[test]
    fn unexpected_equals_sign_before_attribute_name() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("foo=\"bar\" =\"baz\""),
            url: String::new(),
            style: Vec::new(),
        };

        let mut result = HashMap::new();
        result.insert(String::from("foo"), String::from("bar"));
        result.insert(String::from("=\"baz\""), String::new());

        assert_eq!(parser.parse_attributes().unwrap(), result);
    }

    /// attribute names can have : in their name, `xmlns:xlink`
    #[test]
    fn xml_attribute() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("xml:lang='en-US'"),
            url: String::new(),
            style: Vec::new(),
        };

        assert_eq!(
            parser.parse_attr(),
            (String::from("xml:lang"), String::from("en-US"))
        );
    }

    #[test]
    fn attribute_dash() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("v-bind:crates_map='crates' v-bind:tag_filter='tag_filter'>"),
            url: String::new(),
            style: Vec::new(),
        };

        let mut result = HashMap::new();
        result.insert(String::from("v-bind:crates_map"), String::from("crates"));
        result.insert(
            String::from("v-bind:tag_filter"),
            String::from("tag_filter"),
        );

        assert_eq!(parser.parse_attributes(), Some(result));
    }
}
