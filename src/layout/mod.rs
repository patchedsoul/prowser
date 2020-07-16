//! This module contains all code which belongs to layouting.
//! CSS box model. All sizes are in px.

pub mod lbox;

pub use self::BoxType::{AnonymousBlock, BlockNode, InlineNode, TableRowNode};
use crate::stylednode::{Display, StyledNode};

use sdl2::rect::Rect as Sdl_rect;

#[derive(Default, Copy, Clone, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct Dimensions {
    /// Position of the content area relative to the document origin:
    pub content: Rect,
    // Surrounding edges:
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    margin: EdgeSizes,
    /// used width space in inline context
    used_width: f32,
    /// used width space in inline context
    used_height: f32,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Different types a box can have (`Block`, `Inline` or `Anonymous`)
#[derive(Debug)]
pub enum BoxType {
    AnonymousBlock,
    BlockNode(StyledNode),
    TableRowNode(StyledNode),
    /// bool => isInlineBlock
    InlineNode(StyledNode, bool),
}

/// Transform a style tree into a layout tree.
pub fn layout_tree(node: StyledNode, mut containing_block: Dimensions) -> lbox::LBox {
    let containing_root = containing_block;
    // The layout algorithm expects the container height to start at 0.
    containing_block.content.height = 0.0;

    let mut root_box = build_layout_tree(node);
    root_box.layout(
        &mut containing_block,
        &containing_root,
        Some(containing_root.content.height),
    );
    root_box
}

/// Build the tree of `LayoutBoxes`, but don't perform any layout calculations yet.
fn build_layout_tree(style_node: StyledNode) -> lbox::LBox {
    // Create the root box.
    let mut root = lbox::LBox::new(match style_node.display() {
        Display::Block => BlockNode(style_node.clone()),
        Display::TableRow => TableRowNode(style_node.clone()),
        Display::Inline => InlineNode(style_node.clone(), false),
        Display::InlineBlock => InlineNode(style_node.clone(), true),
        Display::None => unreachable!("Root node has `display: none`."),
    });

    let mut block_type = false;
    // check display type of children
    for child in &style_node.children {
        if let Display::Block = child.display() {
            block_type = true;
            break;
        }
    }

    // Create the descendant boxes.
    for child in style_node.children {
        match child.display() {
            Display::Block | Display::TableRow => root.children.push(build_layout_tree(child)),
            Display::Inline | Display::InlineBlock => {
                // if one or several block boxes, create anonymous block
                if block_type {
                    root.get_inline_container()
                        .children
                        .push(build_layout_tree(child))
                } else {
                    root.children.push(build_layout_tree(child))
                }
            }
            Display::None => {} // Don't lay out nodes with `display: none;`
        }
    }
    root
}

impl Rect {
    fn expanded_by(self, edge: EdgeSizes) -> Self {
        Self {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }

    pub fn to_sdlrect(self) -> Sdl_rect {
        Sdl_rect::new(
            self.x as i32,
            self.y as i32,
            self.width as u32,
            self.height as u32,
        )
    }
}

impl Dimensions {
    /// The area covered by the content area plus its padding.
    fn padding_box(self) -> Rect {
        self.content.expanded_by(self.padding)
    }
    /// The area covered by the content area plus padding and borders.
    pub fn border_box(self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }
    /// The area covered by the content area plus padding, borders, and margin.
    pub fn margin_box(self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}
