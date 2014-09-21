//! A simple parser for a tiny subset of HTML and CSS.

use std::ascii::OwnedStrAsciiExt; // for `into_ascii_lower`
use std::collections::hashmap::HashMap;
use std::num::FromStrRadix;

use css::{Stylesheet,Rule,Selector,Simple,SimpleSelector,Declaration,Value,Keyword,Length,Unit,Color,Px};
use dom;

/// Parse an HTML document and return the root element.
pub fn parse_html(source: String) -> dom::Node {
    let mut nodes = Parser { pos: 0u, input: source }.parse_nodes();

    // If the document contains a root element, just return it. Otherwise create one.
    if nodes.len() == 1 {
        nodes.swap_remove(0).unwrap()
    } else {
        dom::elem("html".to_string(), HashMap::new(), nodes)
    }
}

/// Parse a whole CSS stylesheet.
pub fn parse_css(source: String) -> Stylesheet {
    let mut parser = Parser { pos: 0u, input: source };
    Stylesheet { rules: parser.parse_rules() }
}

struct Parser {
    pos: uint,
    input: String,
}

impl Parser {
    /// Read the next character without consuming it.
    fn next_char(&self) -> char {
        self.input.as_slice().char_at(self.pos)
    }

    /// Do the next characters start with the given string?
    fn starts_with(&self, s: &str) -> bool {
        self.input.as_slice().slice_from(self.pos).starts_with(s)
    }

    /// Return true if all input is consumed.
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Return the current character and advance to the next character.
    fn consume_char(&mut self) -> char {
        let range = self.input.as_slice().char_range_at(self.pos);
        self.pos = range.next;
        range.ch
    }

    /// Consume characters until `test` returns false.
    fn consume_while(&mut self, test: |char| -> bool) -> String {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push_char(self.consume_char());
        }
        result
    }

    /// Consume and discard zero or more whitespace
    fn consume_whitespace(&mut self) {
        self.consume_while(|c| c.is_whitespace());
    }

    // HTML parsing

    /// Parse a tag or attribute name.
    fn parse_tag_name(&mut self) -> String {
        self.consume_while(|c| match c {
            'a'..'z' | 'A'..'Z' | '0'..'9' => true,
            _ => false
        })
    }

    /// Parse a single node.
    fn parse_node(&mut self) -> dom::Node {
        match self.next_char() {
            '<' => self.parse_element(),
            _   => self.parse_text()
        }
    }

    /// Parse a text node.
    fn parse_text(&mut self) -> dom::Node {
        dom::text(self.consume_while(|c| c != '<'))
    }

    /// Parse a single element inlcuding its open tag, contents and closing tag.
    fn parse_element(&mut self) -> dom::Node {
        // Opening tag
        assert!(self.consume_char() == '<');
        let tag_name = self.parse_tag_name();
        let attrs = self.parse_attributes();
        assert!(self.consume_char() == '>');

        // Contents
        let children = self.parse_nodes();

        // Closing tag
        assert!(self.consume_char() == '<');
        assert!(self.consume_char() == '/');
        assert!(self.parse_tag_name() == tag_name);
        assert!(self.consume_char() == '>');

        dom::elem(tag_name, attrs, children)
    }

    /// Parse a single name="value" pair.
    fn parse_attr(&mut self) -> (String, String) {
        let name = self.parse_tag_name();
        assert!(self.consume_char() == '=');
        let value = self.parse_attr_value();
        (name, value)
    }

    /// Parse a quoted value.
    fn parse_attr_value(&mut self) -> String {
        let open_quote = self.consume_char();
        assert!(open_quote == '"' || open_quote == '\'');
        let value = self.consume_while(|c| c != open_quote);
        assert!(self.consume_char() == open_quote);
        value
    }

    /// Parse attributes.
    fn parse_attributes(&mut self) -> dom::AttrMap {
        let mut attributes = HashMap::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == '>' {
                break;
            }
            let (name, value) = self.parse_attr();
            attributes.insert(name, value);
        }
        attributes
    }

    /// Parse a sequence of sibling nodes.
    fn parse_nodes(&mut self) -> Vec<dom::Node> {
        let mut nodes = vec!();
        loop {
            self.consume_whitespace();
            if self.eof() || self.starts_with("</") {
                break;
            }
            nodes.push(self.parse_node());
        }
        nodes
    }

    // Parse CSS

    /// Parse a list of rules separated by optional whitespace.
    fn parse_rules(&mut self) -> Vec<Rule> {
        let mut rules = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }
            rules.push(self.parse_rule());
        }
        rules
    }

    /// Parse a rule set: `<selectors> { <declarations> }`.
    fn  parse_rule(&mut self) -> Rule {
        Rule {
            selectors: self.parse_selectors(),
            declarations: self.parse_declarations(),
        }
    }

    // Parse a comma separated list of selectors.
    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            selectors.push(Simple(self.parse_simple_selector()));
            self.consume_whitespace();
            match self.next_char() {
                ',' => {
                    self.consume_char();
                    self.consume_whitespace()
                }
                '{' => break,
                c   => fail!("Unexpected character {} in selector list", c)
            }
        }
        // Sort by specificity (highest first)
        selectors.sort_by(|a, b| b.specificity().cmp(&a.specificity()));
        selectors
    }

    /// Parse one simple selector, e.g.: `type#id.class1.class2.classn`
    fn parse_simple_selector(&mut self) -> SimpleSelector {
        let mut selector = SimpleSelector { tag_name: None, id: None, class: Vec::new() };
        while !self.eof() {
            match self.next_char() {
                '#' => {
                    self.consume_char();
                    selector.id = Some(self.parse_identifier());
                }
                '.' => {
                    self.consume_char();
                    selector.class.push(self.parse_identifier());
                }
                '*' => {
                    // universal selector
                    self.consume_char();
                }
                c if valid_identifier_char(c) => {
                    selector.tag_name = Some(self.parse_identifier());
                }
                _ => break
            }
        }
        selector
    }

    /// Parse a list of declarations enclosed by `{ }`.
    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        assert!(self.consume_char() == '{');
        loop {
            self.consume_whitespace();
            if self.next_char() == '}' {
                self.consume_char();
                break;
            }
            declarations.push(self.parse_declaration());
        }
        declarations
    }

    /// Parse a `<property>: <value>;` declaration.
    fn parse_declaration(&mut self) -> Declaration {
        let property_name = self.parse_identifier();
        self.consume_whitespace();
        assert!(self.consume_char() == ':');
        self.consume_whitespace();
        let value = self.parse_value();
        self.consume_whitespace();
        assert!(self.consume_char() == ';');

        Declaration {
            name: property_name,
            value: value,
        }
    }

    /// Parse a value
    fn parse_value(&mut self) -> Value {
        match self.next_char() {
            '0'..'9' => self.parse_length(),
            '#' => self.parse_color(),
            _   => Keyword(self.parse_identifier())
        }
    }

    fn parse_length(&mut self) -> Value {
        Length(self.parse_float(), self.parse_unit())
    }

    fn parse_float(&mut self) -> f32 {
        let s = self.consume_while(|c| match c {
            '0'..'9' | '.' => true,
            _ => false
        });
        let f: Option<f32> = from_str(s.as_slice());
        f.unwrap()
    }

    fn parse_unit(&mut self) -> Unit {
        match self.parse_identifier().into_ascii_lower().as_slice() {
            "px" => Px,
            _    => fail!("unrecognized unit")
        }
    }

    fn parse_color(&mut self) -> Value {
        assert!(self.consume_char() == '#');
        Color(self.parse_hex_pair(), self.parse_hex_pair(), self.parse_hex_pair(), 255)
    }

    fn parse_hex_pair(&mut self) -> u8 {
        let s = self.input.as_slice().slice(self.pos, self.pos + 2);
        self.pos = self.pos + 2;
        FromStrRadix::from_str_radix(s, 16).unwrap()
    }

    fn parse_identifier(&mut self) -> String {
        self.consume_while(valid_identifier_char)
    }
}

fn valid_identifier_char(c: char) -> bool {
    match c {
        'a'..'z' | 'A'..'Z' | '0'..'9' | '-' | '_' => true,
        _ => false
    }
}
