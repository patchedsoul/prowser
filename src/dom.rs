use std::collections::{HashMap, HashSet};

/// Node in the DOM
/// `node_type`, `children: Vec<Node>`
// https://dom.spec.whatwg.org/#dom-node-nodetype
#[derive(Debug, Clone)]
pub struct Node {
    /// data common to all nodes
    pub children: Vec<Node>,

    /// data specific to each node type
    pub node_type: NodeType,
}

#[derive(Debug, Clone)]
pub enum NodeType {
    /// https://dom.spec.whatwg.org/#text
    Text(Vec<String>),
    /// https://dom.spec.whatwg.org/#element
    Element(ElementData),
}

/// `tag_name: String`, `attributes: AttrMap`
#[derive(Debug, Clone)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: AttrMap,
}

pub type AttrMap = HashMap<String, String>;

impl Node {
    pub fn text(data: String) -> Self {
        Self {
            children: Vec::new(),
            node_type: NodeType::Text(vec![data]),
        }
    }

    pub fn elem(tag_name: String, attributes: AttrMap, children: Vec<Self>) -> Self {
        Self {
            children,
            node_type: NodeType::Element(ElementData {
                tag_name,
                attributes,
            }),
        }
    }
}

impl ElementData {
    pub fn id(&self) -> Option<&String> {
        self.attributes.get("id")
    }

    pub fn classes(&self) -> HashSet<&str> {
        match self.attributes.get("class") {
            Some(classlist) => classlist.split_whitespace().collect(),
            None => HashSet::new(),
        }
    }

    pub fn style(&self) -> Option<&String> {
        self.attributes.get("style")
    }

    pub fn src(&self) -> Option<&String> {
        self.attributes.get("src")
    }

    pub fn get_attribute(&self, attribute: &str) -> Option<&String> {
        self.attributes.get(attribute)
    }
}
