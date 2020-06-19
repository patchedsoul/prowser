mod helper;

use crate::dom;

use std::collections::HashMap;

#[derive(Debug)]
struct Parser {
    pos: usize,
    input: String,
    url: String,
    style: Vec<(String, Option<String>)>,
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

    /// Parse a tag or attribute name.
    fn parse_tag_name(&mut self) -> String {
        self.consume_while(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' => true,
            _ => false,
        })
        .to_ascii_lowercase()
    }

    /// The attribute value can remain unquoted if it doesn't contain ASCII whitespace or any of `"` `'` `` ` `` `=` `<` or `>`.
    /// Otherwise, it has to be quoted using either single or double quotes.
    /// <https://html.spec.whatwg.org/#a-quick-introduction-to-html>
    fn parse_attribute_value(&mut self) -> String {
        self.consume_while(|c| match c {
            ' ' | '"' | '\'' | '`' | '=' | '<' | '>' => false,
            _ => true,
        })
        .to_ascii_lowercase()
    }

    /// Parse a single node.
    fn parse_node(&mut self) -> Option<dom::Node> {
        match self.next_char() {
            Some('<') => self.parse_element(),
            Some(_) => Some(self.parse_text()),
            _ => None,
        }
    }

    /// Parse a text node.
    fn parse_text(&mut self) -> dom::Node {
        let raw_text = self.consume_while(|c| c != '<');
        let mut text = String::new();

        /* FIXME: eventually `let result = s.trim();` before to strip whitespaces. Collapse should already remove all*/

        // collapse whitespace
        let mut whitespace = true;
        for next_char in raw_text.chars() {
            if next_char.is_whitespace() {
                if whitespace {
                    continue;
                }
                whitespace = true;
                text.push(' ');
            } else {
                whitespace = false;
                text.push(next_char);
            }
        }

        // TODO: replace entities
        // https://www.w3schools.com/html/html_entities.asp
        // find all places beginning with & and end with ;
        // for numer version find all places beginning with &# and end with ;
        text = text.replace("&euro;", "€");
        text = text.replace("&copy;", "©");
        text = text.replace("&lt;", "<");
        text = text.replace("&gt;", ">");
        text = text.replace("&amp;", "&");
        text = text.replace("&quot;", "\"");
        text = text.replace("&apos;", "'");
        text = text.replace("&reg;", "®");
        text = text.replace("&trade;", "™");
        text = text.replace("&#9650;", "▲");

        dom::Node::text(text)
    }
}

/// Parse an HTML document and return the root element.
pub fn parse(source: String, url: String) -> (dom::Node, Vec<(String, Option<String>)>) {
    let (mut nodes, style) = Parser {
        pos: 0,
        input: source,
        url,
        style: Vec::new(),
    }
    .parse_nodes();
    // If the document contains a root element, just return it. Otherwise, create one.
    if nodes.len() == 1 {
        (nodes.swap_remove(0), style)
    } else {
        (
            dom::Node::elem("html".to_string(), HashMap::new(), nodes),
            style,
        )
    }
}
