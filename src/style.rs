//! Apply CSS styles to a DOM tree and produce a style tree.

use dom::{Node, Element, ElementData, Text};
use css::{Stylesheet, Rule, Selector, Simple, SimpleSelector, Value, Keyword, Specificity};
use std::collections::hashmap::HashMap;

/// Map from CSS property names to values. A PropertyMap will be associated with a DOM node.
pub type PropertyMap = HashMap<String, Value>;

/// The styled node.
pub struct StyledNode<'a> {
    node: &'a Node, // pointer to a DOM node
    specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

pub type MatchedRule<'a> = (Specificity, &'a Rule);

#[deriving(PartialEq)]
pub enum Display {
    Inline,
    Block,
    DisplayNone,
}

impl<'a> StyledNode<'a> {
    /// Return the specified value of a property if it exists, otherwise `None`.
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.find_equiv(&name).map(|v| v.clone())
    }

    /// Return the specified value of property `name`, or property `fallback_name` if that doesn't
    /// exist. or value `default` if neither does.
    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Value) -> Value {
        self.value(name).unwrap_or_else(|| self.value(fallback_name)
                        .unwrap_or_else(|| default.clone()))
    }

    /// The value of the display property defaults to inline.
    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(Keyword(s)) => match s.as_slice() {
                "block" => Block,
                "none"  => DisplayNone,
                _       => Inline
            },
            _ => Inline
        }
    }
}


fn matches(elem: &ElementData, selector: &Selector) -> bool {
    match *selector {
        Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector)
    }
}

fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    // Check tag selector
    if selector.tag_name.iter().any(|name| elem.tag_name != *name) {
        return false;
    }

    // Check ID selector
    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }

    // Check class selectors
    let elem_classes = elem.classes();
    if selector.class.iter().any(|class| !elem_classes.contains(&class.as_slice())) {
        return false;
    }

    return true;
}

/// If `rule` matches `elem`, return a `MatchedRule`. Otherwise return `None`.
fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    // Find the first (highest-specificity) matching selector.
    rule.selectors
        .iter()
        .find(|selector| matches(elem, *selector))
        .map(|selector| (selector.specificity(), rule))
}

/// Find all CSS rules that match the given element.
fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
    stylesheet.rules.iter().filter_map(|rule| match_rule(elem, rule)).collect()
}

/// Apply styles to a single element, returning the specified values.
fn specified_values(elem: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet);

    // Sort by specificity so that the application of more specific styles override the application
    // of less specific styles.
    rules.sort_by(|&(sa, _), &(sb, _)| sa.cmp(&sb));
    for &(_, rule) in rules.iter() {
        for declaration in rule.declarations.iter() {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    values
}

/// Apply a stylesheet to an entire DOM tree, returning a StyledNode tree.
pub fn style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet) -> StyledNode<'a> {
    StyledNode {
        node: root,
        specified_values: match root.node_type {
            Element(ref elem) => specified_values(elem, stylesheet),
            Text(_) => HashMap::new()
        },
        children: root.children.iter().map(|child| style_tree(child, stylesheet)).collect()
    }
}
