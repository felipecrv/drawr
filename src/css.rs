//! CSS Data Structures
//!
//! Supported selectors:
//!   * Simple selector:
//!     - tag
//!     - .class
//!     - #id
//!     - *
//!     - combination of all the above (e.g. tag#id.class1.class2)

#[deriving(Show)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[deriving(Show)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[deriving(Show)]
pub enum Selector {
    Simple(SimpleSelector),
}

#[deriving(Show)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>
}

#[deriving(Show)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

#[deriving(Show, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Color(u8, u8, u8, u8), // RGBA
    Length(f32, Unit),
}

#[deriving(Show, Clone, PartialEq)]
pub enum Unit {
    Px // Pixels
}

pub type Specificity = (uint, uint, uint);

impl Value {
    /// Return the size of a length in px, or zero for non-lengths.
    pub fn to_px(&self) -> f32 {
        match *self {
            Length(f, Px) => f,
            _ => 0.0
        }
    }
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let Simple(ref simple) = *self;
        let a = simple.id.iter().len();
        let b = simple.class.len();
        let c = simple.tag_name.iter().len();
        (a, b, c)
    }
}
