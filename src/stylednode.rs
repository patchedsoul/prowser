use crate::css::Value;
use crate::dom;
use crate::style::PropertyMap;

/// A node with associated style data.
#[derive(Debug, Clone)]
pub struct StyledNode {
    pub node: dom::Node,
    /// css style
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode>,
}

/// Posible `display: ` values
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    None,
    TableRow,
}

impl StyledNode {
    // could be moved to `impl Node`?
    pub fn finde_node(
        &self,
        tag_name: &str,
        attribute: Option<(&str, &str)>,
    ) -> Option<&dom::Node> {
        if let dom::NodeType::Element(el) = &self.node.node_type {
            if el.tag_name == tag_name {
                if let Some(attribute) = attribute {
                    if let Some(key_pair) = el.attributes.get_key_value(attribute.0) {
                        if key_pair.1 == attribute.1 {
                            return Some(&self.node);
                        }
                    }
                } else {
                    return Some(&self.node);
                }
            }
        }
        for child in &self.children {
            if let Some(node) = child.finde_node(tag_name, attribute) {
                return Some(node);
            }
        }

        None
    }

    /// Return the specified value of a property if it exists, otherwise `None`.
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).cloned()
    }

    /// Return the specified value of property `name`, or `default` if that doesn't exist.
    pub fn lookup(&self, name: &str, default: &Value) -> Value {
        self.value(name).unwrap_or_else(|| default.clone())
    }

    /// The value of the `display` property (defaults to `inline`).
    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(Value::Keyword(s)) => match &*s {
                "block" | "list-item" | "table" | "grid" | "flex" => Display::Block,
                "table-row" => Display::TableRow,
                "none" => Display::None,
                "inline-block" => Display::InlineBlock,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }

    /// get attributes specified on the element (like `width="20"`)
    pub fn attribute(&self, attribute: &str) -> Option<&String> {
        if let dom::NodeType::Element(element) = &self.node.node_type {
            return element.get_attribute(attribute);
        }
        None
    }
}
