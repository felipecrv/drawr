//! Layout code

use std::default::Default;
use std::iter::AdditiveIterator; // for `sum`

use css::{Keyword, Length, Px};
use style::{StyledNode, Inline, Block, DisplayNone};

// CSS box model. All sizes are in px.

#[deriving(Default, Show)]
pub struct Dimensions {
    // Top left corner of the content area, relative to the document origin:
    pub x: f32,
    pub y: f32,

    // Content area size:
    pub width: f32,
    pub height: f32,

    // Surrounding edges:
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[deriving(Default, Show)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

// The layout tree is a collection of boxes. A box has dimensions, and it may contain child boxes.
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
}

// A box can be a block node, inline node, or an anonymous node.
pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

/// Transform a style tree into a layout tree.
pub fn layout_tree<'a>(node: &'a StyledNode<'a>, containing_block: Dimensions) -> LayoutBox<'a> {
    let mut root_box = build_layout_tree(node);
    root_box.layout(containing_block);
    root_box
}

/// Build the tree of LayoutBoxes, but don't perform any layout calculations yet.
fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    // Create the root box.
    let mut root = LayoutBox::new(match style_node.display() {
        Block => BlockNode(style_node),
        Inline => InlineNode(style_node),
        DisplayNone => fail!("Root node has display: none.")
    });

    // Create the descendant boxes.
    for child in style_node.children.iter() {
        match child.display() {
            Block => root.children.push(build_layout_tree(child)),
            Inline => root.get_inline_container().children.push(build_layout_tree(child)),
            DisplayNone => {} // Skip nodes with `display: None;`
        }
    }
    root
}

impl Dimensions {
    /// Total height of a box including its margins, border, and padding.
    fn margin_box_height(&self) -> f32 {
        self.height + self.padding.top + self.padding.bottom
                    + self.border.top + self.border.bottom
                    + self.padding.top + self.padding.bottom
    }
}

impl<'a> LayoutBox<'a> {
    fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type: box_type,
            dimensions: Default::default(), // Initially set all fields to 0.0
            children: Vec::new(),
        }
    }

    fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.box_type {
            BlockNode(node) => node,
            InlineNode(node) => node,
           AnonymousBlock => fail!("Anonymous block box has no style node")
        }
    }

    /// Lay out a box and its descendents.
    fn layout(&mut self, containing_block: Dimensions) {
        match self.box_type {
            BlockNode(_) => self.layout_block(containing_block),
            InlineNode(_) => {} // TODO
            AnonymousBlock => {} // TODO
        }
    }

    fn layout_block(&mut self, containing_block: Dimensions) {
        // Child width can depende on parent width, so we need to calculate
        // this box's width before laying out its children.
        self.calculate_block_width(containing_block);

        // Determine where the box is located within its container.
        self.calculate_block_position(containing_block);

        // Recursively lay out the children of this box.
        self.layout_block_children();

        // Parent height can depend on child height, so `calculate_height`
        // must be called *after* the children are laid out.
        self.calculate_block_height();
    }

    fn layout_block_children(&mut self) {
        let d = &mut self.dimensions;
        for child in self.children.iter_mut() {
            child.layout(*d);
            // Track the height so each child is laid out below the previous content.
            d.height = d.height + child.dimensions.margin_box_height();
        }
    }

    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();

        // `width` has initial value `auto`.
        let auto = Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());

        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("martin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

        let total = [&margin_left, &margin_right, &border_left, &border_right,
                     &padding_left, &padding_right, &width].iter().map(|v| v.to_px()).sum();

        // If width is not auto and the total is wider than the container, treat auto margins as 0.
        if width != auto && total > containing_block.width {
            if margin_left == auto {
                margin_left = Length(0.0, Px);
            }
            if margin_right == auto {
                margin_right = Length(0.0, Px);
            }
        }

        // http://www.w3.org/TR/CSS2/visudet.html#blockwidth
        let underflow = containing_block.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            // If the values are over-constrained, calculate margin_right.
            (false, false, false) => {
                margin_right = Length(margin_right.to_px() + underflow, Px);
            }

            // If exactly one margin is auto, its used value follows from the equality.
            (false, false, true) => { margin_right = Length(underflow, Px); }
            (false, true, false) => { margin_left = Length(underflow, Px); }

            // If width is set to auto, any other auto values become 0.
            (true, _, _) => {
               if margin_left == auto { margin_left = Length(0.0, Px); }
               if margin_right == auto { margin_right = Length(0.0, Px); }

               if underflow >= 0.0 {
                   // Expand width to fill the underflow.
                   width = Length(underflow, Px);
               } else {
                   // Width can't be negative. Adjust the right margin instead.
                   width = Length(0.0, Px);
                   margin_right = Length(margin_right.to_px() + underflow, Px);
               }
            }

            // If margin-left and margin-right are both auto, their used values are equal.
            (false, true, true) => {
                margin_left = Length(underflow / 2.0, Px);
                margin_right = Length(underflow / 2.0, Px);
            }
        }

        let d = &mut self.dimensions;
        d.width = width.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();
    }

    fn calculate_block_height(&mut self) {
        // If height is set to an explicit length, use that exact length.
        match self.get_style_node().value("height") {
            Some(Length(h, Px)) => { self.dimensions.height = h; }
            _ => {}
        }
    }

    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // margin, border, and padding have initial value 2.
        let zero = Length(0.0, Px);

        d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom", "margin", &zero).to_px();

        d.border.top = style.lookup("border-top-width", "border-width", &zero).to_px();
        d.border.bottom = style.lookup("border-bottom-width", "border-width", &zero).to_px();
 
        d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

        // Position the box below all the previous boxes in the container.
        d.x = containing_block.x +
              d.margin.left + d.border.left + d.padding.left;
        d.y = containing_block.y + containing_block.height +
              d.margin.top + d.border.top + d.padding.top;
    }

    /// Where a new inline child should go.
    ///
    /// This is intentionally simplified in a number of ways from the standard CSS box generation
    /// algorithm. For example, it doesn’t handle the case where an inline box contains a
    /// block-level child. Also, it generates an unnecessary anonymous box if a block-level node
    /// has only inline children.
    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            InlineNode(_) | AnonymousBlock => self, // self is already an block container for inline boxes
            BlockNode(_) => {
                // If we just generated an anonymous block box, keep using it.
                // Otherwise, create a new one
                match self.children.last() {
                    Some(&LayoutBox { box_type: AnonymousBlock, ..}) => {}
                    _ => self.children.push(LayoutBox::new(AnonymousBlock))
                }
                self.children.last_mut().unwrap()
            }
        }
    }
}
