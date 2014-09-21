//! Basic DOM data structures

use std::collections::hashmap::{HashMap, HashSet};

pub struct Node {
    // data common to all nodes:
    pub children: Vec<Node>,

    // specific data to each node type:
    pub node_type: NodeType,
}

#[deriving(Show)]
pub enum NodeType {
    Text(String),
    Element(ElementData),
}

#[deriving(Show)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: AttrMap,
}

pub type AttrMap = HashMap<String, String>;

// Constructors

pub fn text(data: String) -> Node {
    Node { children: vec![], node_type: Text(data) }
}

pub fn elem(name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
    Node {
        children: children,
        node_type: Element(ElementData {
            tag_name: name,
            attributes: attrs,
        })
    }
}

// Element methods

impl ElementData {
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.find_equiv(&key)
    }

    pub fn id(&self) -> Option<&String> {
        self.get_attribute("id")
    }

    pub fn classes(&self) -> HashSet<&str> {
        match self.get_attribute("class") {
            Some(classlist) => classlist.as_slice().split(' ').collect(),
            None => HashSet::new()
        }
    }
}
