use crate::css::{self, ChainedSelector, Rule, SimpleSelector, Specificity, Stylesheet, Value};
use crate::dom::{ElementData, Node, NodeType};
use crate::stylednode::StyledNode;

use std::collections::HashMap;

/// Map from CSS property names to values.
pub type PropertyMap = HashMap<String, Value>;

/// A single CSS rule and the specificity of its most specific matching selector.
type MatchedRule<'a> = (Specificity, &'a Rule);

/// Apply a stylesheet to an entire DOM tree, returning a `StyledNode` tree.
pub fn style_tree(
    root: Node,
    stylesheets: &[Stylesheet],
    parent_style: &PropertyMap,
    combinators: Vec<Vec<&ElementData>>,
    url: &str,
) -> StyledNode {
    let mut combinators = combinators;

    let specified_values = match root.node_type {
        NodeType::Element(ref elem) => {
            combinators.last_mut().unwrap().push(elem);
            let mut values = specified_values(elem, stylesheets, &combinators, url.to_string());
            values = inherit_values(parent_style, values);
            values
        }
        NodeType::Text(..) => inherit_values(parent_style, HashMap::new()),
    };

    combinators.push(Vec::new());

    StyledNode {
        children: root
            .children
            .iter()
            .map(|child| {
                let child_node = style_tree(
                    child.to_owned(),
                    stylesheets,
                    &specified_values,
                    combinators.clone(),
                    url,
                );
                if let NodeType::Element(ref elem) = child.node_type {
                    combinators.last_mut().unwrap().push(elem);
                }
                child_node
            })
            .collect(),
        specified_values,
        node: root,
    }
}

/// Apply styles to a single element, returning the specified styles.
fn specified_values(
    elem: &ElementData,
    stylesheets: &[Stylesheet],
    combinators: &[Vec<&ElementData>],
    url: String,
) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = Vec::new();
    for stylesheet in stylesheets {
        for rule in matching_rules(stylesheet, combinators) {
            rules.push(rule);
        }
    }

    let mut declarations = Vec::new();

    for (mut specificity, rule) in rules {
        for declaration in &rule.declarations {
            specificity.0 = declaration.important;
            declarations.push((specificity, declaration.clone()));
        }
    }

    // css rules from the `style` attribute
    if let Some(style) = elem.style() {
        let mut parser = css::parser::Parser {
            pos: 0,
            input: style.to_string(),
            url,
        };

        for declaration in parser.parse_declarations() {
            let specificity = (declaration.important, true, 0, 0, 0);
            declarations.push((specificity, declaration));
        }
    }

    // Go through the declarations from lowest to highest specificity.
    declarations.sort_by(|&(a, _), &(b, _)| a.cmp(&b));

    for (_, declaration) in declarations {
        values.insert(declaration.name, declaration.value);
    }

    values
}

/// Returns inherit properties.
fn inherit_values(parent_style: &PropertyMap, mut own_style: PropertyMap) -> PropertyMap {
    // `inherit` keyword
    for (name, value) in &mut own_style {
        if value == &mut css::Value::Keyword(String::from("inherit")) {
            *value = parent_style
                .get(name)
                .unwrap_or(&css::Value::Keyword(String::from("initial")))
                .clone();
        }
    }

    // `inherit` values
    let inherit_values = [
        "border-collapse",     // separate
        "border-spacing",      // 0
        "caption-side",        // top
        "color",               // 0,0,0,255
        "cursor",              // auto
        "direction",           // ltr
        "empty-cells",         // show
        "font-family",         // user-agent
        "font-size",           // medium: 16px
        "font-style",          // normal
        "font-variant",        // normal
        "font-weight",         // normal
        "letter-spacing",      // normal
        "line-height",         // normal
        "list-style-image",    // none
        "list-style-position", // outside
        "list-style-type",     // disc
        "quotes",              // user-agent
        "text-align",          // start
        "text-indent",         // 0
        "text-transform",      // none
        "visibility",          // visible
        "white-space",         // normal
        "word-spacing",        // normal
        "text-decoration",     // FIXME: should not inherited, but apply to the Text node below it
    ];

    for &value in &inherit_values {
        if let Some(parent_value) = parent_style.get(value) {
            own_style
                .entry(value.into())
                .or_insert_with(|| parent_value.clone());
        }
    }

    own_style
}

/// Find all CSS rules that match the given element.
fn matching_rules<'a>(
    stylesheet: &'a Stylesheet,
    combinators: &[Vec<&ElementData>],
) -> Vec<MatchedRule<'a>> {
    // TODO: "For now, we just do a linear scan of all the rules. For large
    // documents, it would be more efficient to store the rules in hash tables
    // based on tag name, id, class, etc."
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(rule, combinators))
        .collect()
}

/// If `rule` matches `elem`, return a `MatchedRule`. Otherwise return `None`.
fn match_rule<'a>(rule: &'a Rule, combinators: &[Vec<&ElementData>]) -> Option<MatchedRule<'a>> {
    // Find the first (most specific) matching selector.
    rule.selectors.iter().find_map(|selector| {
        if matches(selector, combinators) {
            Some((selector.specificity(), rule))
        } else {
            None
        }
    })
}

/// Selector matching:
fn matches(selector: &ChainedSelector, combinators: &[Vec<&ElementData>]) -> bool {
    let mut combinators = combinators.to_owned();
    combinators.reverse();

    matches_chained_selector(1, 0, &selector.selectors, &combinators).is_some()
}

/// Checks if a `ChainedSelector` matches.
///
/// [w3](https://www.w3.org/TR/selectors-3/#combinators)
fn matches_chained_selector(
    mut parent_index: usize,
    mut sibling_index: usize,
    selectors: &[(SimpleSelector, char)],
    combinators: &[Vec<&ElementData>],
) -> Option<()> {
    'outer: for (index, (simple, kombinator)) in selectors.iter().rev().enumerate() {
        match kombinator {
            ' ' => {
                // any predecessor
                loop {
                    parent_index += 1;
                    if matches_simple_selector(combinators.get(parent_index - 1)?.last()?, simple) {
                        let len = selectors.len();
                        if matches_chained_selector(
                            parent_index,
                            sibling_index,
                            &selectors[..len - index - 1],
                            combinators,
                        )
                        .is_some()
                        {
                            break 'outer;
                        }
                    }
                }
            }
            '>' => {
                // direct parent
                if !matches_simple_selector(combinators.get(parent_index)?.last()?, simple) {
                    return None;
                }
                parent_index += 1;
            }
            '+' => {
                // direct sibling
                let vec = combinators.get(parent_index - 1)?;

                if vec.len() < 2 + sibling_index
                    || !matches_simple_selector(vec.get(vec.len() - 2 - sibling_index)?, simple)
                {
                    return None;
                }
                sibling_index += 1;
            }
            '~' => {
                // any Sibling
                loop {
                    sibling_index += 1;
                    if matches_simple_selector(
                        combinators.get(parent_index - 1)?.get(sibling_index - 1)?,
                        simple,
                    ) {
                        let len = selectors.len();
                        if matches_chained_selector(
                            parent_index,
                            sibling_index - 1,
                            &selectors[..len - index - 1],
                            combinators,
                        )
                        .is_some()
                        {
                            break 'outer;
                        }
                    }
                }
            }
            '-' => {
                // element it self
                if !matches_simple_selector(combinators.first()?.last()?, simple) {
                    return None;
                }
            }
            c => panic!("unknown char as combinator: {}", c),
        }
    }

    // We didn't find any non-matching selector.
    Some(())
}

/// Checks if a `SimpleSelector` matches.
/// All criterias have to match. If any doesn't, the selctor doesn't.
fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    // Check type selector
    if selector
        .tag_name
        .iter()
        .any(|name| elem.tag_name != *name.to_ascii_lowercase())
    {
        return false;
    }

    // Check ID selector
    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }

    // Check class selectors
    let elem_classes = elem.classes();
    if selector
        .class
        .iter()
        .any(|class| !elem_classes.contains(&**class))
    {
        return false;
    }

    // Check attribute selector
    // https://www.w3.org/TR/selectors-3/#attribute-selectors
    let attributes = &elem.attributes;
    for (identifier, specifier, value) in &selector.attribute {
        match specifier {
            '=' => {
                if attributes.get(identifier) != Some(value) {
                    return false;
                }
            }
            '~' => match attributes.get(identifier) {
                Some(attribute_value) => {
                    if !attribute_value.split_whitespace().any(|x| x == *value) {
                        return false;
                    }
                }
                None => return false,
            },
            '|' => match attributes.get(identifier) {
                Some(attribute_value) => {
                    let mut value_slash = value.clone();
                    value_slash.push('-');
                    if !attribute_value.starts_with(&value_slash) && attribute_value != value {
                        return false;
                    }
                }
                None => return false,
            },
            '^' => match attributes.get(identifier) {
                Some(attribute_value) => {
                    if !attribute_value.starts_with(value) || value.is_empty() {
                        return false;
                    }
                }
                None => return false,
            },
            '$' => match attributes.get(identifier) {
                Some(attribute_value) => {
                    if !attribute_value.ends_with(value) || value.is_empty() {
                        return false;
                    }
                }
                None => return false,
            },
            '*' => match attributes.get(identifier) {
                Some(attribute_value) => {
                    if !attribute_value.contains(value) || value.is_empty() {
                        return false;
                    }
                }
                None => return false,
            },
            _ => {
                if !attributes.contains_key(&*identifier) {
                    return false;
                }
            }
        }
    }

    // We didn't find any non-matching selector components.
    true
}
