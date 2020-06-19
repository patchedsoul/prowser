mod color;
mod helper;

use crate::css::{
    media_query, valid_identifier_char, ChainedSelector, Declaration, Rule, SimpleSelector, Value,
};

pub struct Parser {
    pub input: String,
    pub pos: usize,
    pub url: String,
}

impl Parser {
    /// Parse a list of rule sets, separated by optional whitespace and comments.
    pub fn parse_rules(&mut self, dimensions: (u32, u32)) -> Vec<Rule> {
        let mut rules = Vec::new();
        loop {
            self.consume_blank();
            if self.eof() {
                break;
            }

            if let Some('@') = self.next_char() {
                if self.starts_with("@media") {
                    let query_condition = self.consume_while(|c| c != '{'); // query condition
                    self.consume_char(); // {

                    let mut query_rules = self.parse_rules(dimensions); // rules inside the query

                    let mut parser = media_query::parser::Parser {
                        pos: 0,
                        input: query_condition[6..].to_string(),
                    };

                    if parser.matches(dimensions) {
                        rules.append(&mut query_rules);
                    }
                } else if self.starts_with("@import") {
                    self.consume_while(|c| c != ';');
                    self.consume_char(); // ;
                    continue;
                } else {
                    // FIXME: parse other @ functions like keyframe and font
                    self.consume_while(|c| {
                        c != '{' && c != '[' && c != '(' && c != '\'' && c != '"' && c != '}'
                    });
                    if let Some(c) = self.next_char() {
                        let rchar = match c {
                            '}' => {
                                self.consume_char();
                                break;
                            }
                            '{' => '}',
                            '[' => ']',
                            '(' => ')',
                            '\'' => '\'',
                            '"' => '"',
                            _ => ' ',
                        };
                        self.consume_while(|c| c != rchar);
                        if rchar == '}' {
                            self.consume_char();
                        }
                    }

                    continue;
                }
                self.consume_char(); // }
                continue;
            } else if let Some('}') = self.next_char() {
                self.consume_char(); // }
                break;
            }

            if let Some(rule) = self.parse_rule() {
                rules.push(rule);
            }
        }
        rules
    }

    /// Parse a rule set: `<selectors> { <declarations> }`.
    fn parse_rule(&mut self) -> Option<Rule> {
        if let Some(selectors) = self.parse_selectors() {
            Some(Rule {
                selectors,
                declarations: {
                    self.consume_char(); // {
                    let declarations = self.parse_declarations();
                    self.consume_char(); // }
                    if declarations.is_empty() {
                        return None;
                    } else {
                        declarations
                    }
                },
            })
        } else {
            self.consume_while(|c| c != '{');
            self.consume_char(); // {
            self.parse_declarations();
            self.consume_char(); // }
            None
        }
    }

    /// Parse a comma-separated list of selectors. `<selector>, <selector>`.
    fn parse_selectors(&mut self) -> Option<Vec<ChainedSelector>> {
        let mut selectors = Vec::new();
        loop {
            selectors.push(self.parse_selector());
            match self.next_char() {
                Some(',') => {
                    self.consume_char();
                    self.consume_blank();
                }
                Some('{') => break,
                _ => return None,
            }
        }
        // Return selectors with highest specificity first, for use in matching.
        selectors.sort_by(|a, b| b.specificity().cmp(&a.specificity()));
        Some(selectors)
    }

    /// Parse a selector `<selector#id tag.class>`
    fn parse_selector(&mut self) -> ChainedSelector {
        let mut chained_selector = ChainedSelector {
            selectors: Vec::new(),
        };

        loop {
            let simple_selector = self.parse_simple_selector();
            self.consume_blank();

            match self.next_char() {
                Some('#') | Some('.') | Some('*') | Some('a'..='z') | Some('A'..='Z')
                | Some('0'..='9') => {
                    chained_selector.selectors.push((simple_selector, ' '));
                }
                Some('>') | Some('~') | Some('+') => {
                    chained_selector
                        .selectors
                        .push((simple_selector, self.consume_char().unwrap()));
                    self.consume_blank();
                }
                _ => {
                    // End of Selectors, mark with -
                    chained_selector.selectors.push((simple_selector, '-'));
                    break;
                }
            }
        }

        chained_selector
    }

    /// Parse one `SimpleSelector`, e.g.: `type#id.class1.class2.class3[type=hidden}`
    fn parse_simple_selector(&mut self) -> SimpleSelector {
        let mut selector = SimpleSelector {
            tag_name: None,
            id: None,
            class: Vec::new(),
            attribute: Vec::new(),
        };
        while !self.eof() {
            match self.next_char().unwrap() {
                '#' => {
                    self.consume_char();
                    selector.id = Some(self.parse_identifier());
                }
                '.' => {
                    self.consume_char();
                    selector.class.push(self.parse_identifier());
                }
                '*' => {
                    self.consume_char();
                }
                ':' => {
                    self.consume_char();
                    if let Some(':') = self.next_char() {
                        // TODO: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-elements
                        self.consume_char();
                    }
                    // TODO: https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes

                    if self.parse_identifier() == "link" {
                        // FIXME: workaround for link and a, until a better solution
                        selector.tag_name = Some(String::from("a"));
                    } else {
                        // FIXME: prevent rules from getting applied to any element
                        selector.class.push(String::from("-"));
                    }
                }
                '[' => {
                    self.consume_char(); // [
                    self.consume_blank();

                    let identifier = self.parse_identifier().to_ascii_lowercase();
                    let specifier;
                    let mut value = String::new();

                    match self.next_char() {
                        Some('=') => {
                            specifier = self.consume_char().unwrap(); // =
                            value = self.parse_attribute_value();
                        }
                        Some('~') | Some('|') | Some('^') | Some('$') | Some('*') => {
                            specifier = self.consume_char().unwrap();
                            self.consume_char(); // =

                            value = self.parse_attribute_value();
                        }
                        _ => specifier = ' ',
                    }

                    selector.attribute.push((identifier, specifier, value));

                    self.consume_char(); // ]
                }
                c if valid_identifier_char(c) => {
                    selector.tag_name = Some(self.parse_identifier().to_ascii_lowercase());
                }
                _ => break,
            }
        }
        selector
    }

    /// parses a value which is everything not `]` or everthing in `'`/`"`
    fn parse_attribute_value(&mut self) -> String {
        let value;

        if let Some('"') | Some('\'') = self.next_char() {
            let open_quote = self.consume_char().unwrap(); // ' | "
            value = self.consume_while(|c| c != open_quote);
            self.consume_char(); // open_quote
        } else {
            value = self.consume_while(|c| c != ']');
        }
        value
    }

    /// Parse a list of declarations without `{ ... }`.
    pub fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        loop {
            self.consume_blank();
            if self.eof() || self.next_char().unwrap() == '}' {
                break;
            }
            if let Some(parsed_declarations) = self.parse_declaration() {
                for declaration in parsed_declarations {
                    declarations.push(declaration);
                }
            }
        }
        declarations
    }

    /// Parse one `<property>: <value>;` declaration.
    /// Split if `<property>: <value> <value>;`
    fn parse_declaration(&mut self) -> Option<Vec<Declaration>> {
        let property_name = self.parse_identifier();

        if !property_name.is_empty() {
            self.consume_blank();

            if let Some(':') = self.next_char() {
                self.consume_char(); // :
                self.consume_blank();

                let mut declarations = Vec::new();

                let (values, important) = match self.parse_values() {
                    Some(values) => values,
                    None => return None,
                };

                // custom property/variable
                if property_name.starts_with("--") {
                    println!("custom property found: {}", &property_name);
                    // TODO: save custom property
                    return None;
                }

                let array = [
                    "margin",
                    "padding",
                    "border-width",
                    "border-color",
                    "border-style",
                ];
                if array.contains(&&*property_name) {
                    let value_numbers = match values.len() {
                        1 => (0, 0, 0, 0),
                        2 => (0, 1, 0, 1),
                        3 => (0, 1, 2, 1),
                        4 => (0, 1, 2, 3),
                        _ => return None,
                    };

                    let mut name1;
                    let mut name2;
                    let mut name3;
                    let mut name4;

                    if property_name.starts_with("border") {
                        let postfix = match &*property_name {
                            "border-width" => "-width",
                            "border-color" => "-color",
                            "border-style" => "-style",
                            _ => unreachable!(),
                        };

                        name1 = format!("border-top{}", postfix);
                        name2 = format!("border-right{}", postfix);
                        name3 = format!("border-bottom{}", postfix);
                        name4 = format!("border-left{}", postfix);
                    } else {
                        name1 = property_name.clone();
                        name2 = property_name.clone();
                        name3 = property_name.clone();
                        name4 = property_name;

                        name1.push_str("-top");
                        name2.push_str("-right");
                        name3.push_str("-bottom");
                        name4.push_str("-left");
                    }

                    declarations.push(Declaration {
                        name: name1,
                        value: values[value_numbers.0].clone(),
                        important,
                    });
                    declarations.push(Declaration {
                        name: name2,
                        value: values[value_numbers.1].clone(),
                        important,
                    });
                    declarations.push(Declaration {
                        name: name3,
                        value: values[value_numbers.2].clone(),
                        important,
                    });
                    declarations.push(Declaration {
                        name: name4,
                        value: values[value_numbers.3].clone(),
                        important,
                    });
                } else if property_name == "border" {
                    for value in values {
                        let postfix = match value {
                            Value::Color(..) => "-color",
                            Value::Length(..) => "-width",
                            Value::Keyword(ref keyword) => {
                                if keyword == "none" || keyword == "hidden" {
                                    return None;
                                }
                                "-style"
                            }
                            _ => return None,
                        };
                        let name1 = format!("border-top{}", postfix);
                        let name2 = format!("border-right{}", postfix);
                        let name3 = format!("border-bottom{}", postfix);
                        let name4 = format!("border-left{}", postfix);

                        declarations.push(Declaration {
                            name: name1,
                            value: value.clone(),
                            important,
                        });
                        declarations.push(Declaration {
                            name: name2,
                            value: value.clone(),
                            important,
                        });
                        declarations.push(Declaration {
                            name: name3,
                            value: value.clone(),
                            important,
                        });
                        declarations.push(Declaration {
                            name: name4,
                            value,
                            important,
                        });
                    }
                } else if property_name == "background" {
                    for value in values {
                        match value {
                            Value::Color(..) => {
                                declarations.push(Declaration {
                                    name: String::from("background-color"),
                                    value,
                                    important,
                                });
                            }
                            Value::Url(..) => {
                                declarations.push(Declaration {
                                    name: String::from("background-image"),
                                    value,
                                    important,
                                });
                            }
                            _ => {}
                        }
                    }
                } else if property_name == "font-family" {
                    let mut family = String::new();
                    for value in values {
                        if let Value::Keyword(keyword) = value {
                            if let "sans-serif" | "serif" | "monospace" = &*keyword {
                                family = keyword;
                            }
                            break;
                        }
                    }
                    declarations.push(Declaration {
                        name: property_name,
                        value: Value::Keyword(family),
                        important,
                    });
                } else if values.len() > 1 {
                    return None;
                } else {
                    declarations.push(Declaration {
                        name: property_name,
                        value: values[0].clone(),
                        important,
                    });
                }

                return Some(declarations);
            }
        }
        loop {
            self.consume_while(|c| {
                c != '{' && c != '[' && c != '(' && c != '\'' && c != '"' && c != ';' && c != '}'
            });
            if let Some(c) = self.next_char() {
                let rchar = match c {
                    ';' | '}' => {
                        self.consume_char();
                        return None;
                    }
                    '{' => '}',
                    '[' => ']',
                    '(' => ')',
                    '\'' => '\'',
                    '"' => '"',
                    _ => ' ',
                };
                self.consume_while(|c| c != rchar);
            }
        }
    }
}

#[cfg(test)]
mod rules {
    use super::*;

    #[test]
    fn import() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("@import url('bluish.css') speech;"),
            url: String::new(),
        };
        assert_eq!(parser.parse_rules((0, 0)).len(), 0);
        assert_eq!(parser.pos, 33);
    }

    #[test]
    fn media_query() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("@media screen {.b {color:red}}.a{color: blue}"),
            url: String::new(),
        };
        assert_eq!(parser.parse_rules((0, 0)).len(), 2);

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("@media print {.b {color:red}}.a{color: blue}"),
            url: String::new(),
        };
        assert_eq!(parser2.parse_rules((0, 0)).len(), 1);
    }
}
