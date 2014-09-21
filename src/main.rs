extern crate getopts;

use getopts::{optopt,getopts};
use std::default::Default;
use std::io::fs::File;
use std::os::args;

mod css;
mod dom;
mod layout;
mod parser;
mod style;

fn main() {
    // Parse command-line options:
    let opts = [
        optopt("h", "html", "HTML document", "FILENAME"),
        optopt("c", "css", "CSS stylesheet", "FILENAME"),
    ];
    let matches = match getopts(args().tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_string())
    };

    // Read input files:
    let read_source = |arg_filename: Option<String>, default_filename: &str| {
        let path = match arg_filename {
            Some(ref filename) => filename.as_slice(),
            None => default_filename,
        };
        File::open(&Path::new(path)).read_to_string().unwrap()
    };
    let html = read_source(matches.opt_str("h"), "examples/test.html");
    let css  = read_source(matches.opt_str("c"), "examples/test.css");

    // Since we don't have an actual window, hard-code the "viewport" size.
    let initial_containing_block = layout::Dimensions {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
        padding: Default::default(),
        border: Default::default(),
        margin: Default::default(),
    };

    // Parsing and rendering:
    let root_node = parser::parse_html(html);
    let stylesheet = parser::parse_css(css);
    let style_root = style::style_tree(&root_node, &stylesheet);
    let layout_root = layout::layout_tree(&style_root, initial_containing_block);

    // Debug output:
    println!("{}", layout_root.dimensions);
}
