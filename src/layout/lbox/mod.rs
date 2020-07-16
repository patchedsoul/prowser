mod block;
mod inline;

use crate::dom;
use crate::layout::{
    AnonymousBlock, BlockNode, BoxType, Dimensions, InlineNode, StyledNode, TableRowNode,
};

use std::default::Default;

/// Posible `position: ` values
#[derive(Debug)]
enum Position {
    //Absolute,
    Fixed,
    //Relative,
    Static,
    //Sticky,
}

/// A node in the layout tree.
#[derive(Debug)]
pub struct LBox {
    pub box_type: BoxType,
    pub children: Vec<LBox>,
    pub dimensions: Dimensions,
    position: Position,
}

impl LBox {
    pub fn new(box_type: BoxType) -> Self {
        Self {
            box_type,
            dimensions: Dimensions::default(),
            children: Vec::new(),
            position: Position::Static,
        }
    }

    pub fn finde_box_id(&self, id: &str) -> Option<&Self> {
        if let BoxType::BlockNode(styled_node) | BoxType::InlineNode(styled_node, _) =
            &self.box_type
        {
            if let dom::NodeType::Element(el) = &styled_node.node.node_type {
                if let Some(element_id) = el.id() {
                    if element_id == id {
                        return Some(self);
                    }
                }
            }
        }

        for child in &self.children {
            if let Some(lbox) = child.finde_box_id(id) {
                return Some(lbox);
            }
        }

        None
    }

    /// find clicked element node
    pub fn find_coordinate_element(&self, x: i32, y: i32) -> Option<&Self> {
        for child in self.children.iter().rev() {
            if let BoxType::BlockNode(styled_node) | BoxType::InlineNode(styled_node, _) =
                &child.box_type
            {
                // only check `Element`s, not `Text` nodes
                if let dom::NodeType::Element(_) = &styled_node.node.node_type {
                    if let Some(lbox) = child.find_coordinate_element(x, y) {
                        return Some(lbox);
                    }
                }
            }
        }

        let padding_box = self.dimensions.padding_box();
        if padding_box.x as i32 <= x
            && (padding_box.x + padding_box.width) as i32 > x
            && padding_box.y as i32 <= y
            && (padding_box.y + padding_box.height) as i32 > y
        {
            Some(self)
        } else {
            None
        }
    }

    /// returns node with `specified_values` (aka css style) and children
    fn get_style_node(&self) -> &StyledNode {
        match self.box_type {
            TableRowNode(ref node) | BlockNode(ref node) | InlineNode(ref node, _) => node,
            AnonymousBlock => unreachable!("Anonymous block box has no style node"),
        }
    }

    /// Lay out a box and its descendants.
    pub fn layout(
        &mut self,
        containing_block: &mut Dimensions,
        root_block: &Dimensions,
        parent_height: Option<f32>,
    ) {
        match self.box_type {
            AnonymousBlock => self.layout_anonymous(containing_block, root_block),
            BlockNode(..) => self.layout_block(containing_block, root_block, parent_height),
            InlineNode(_, inline_block) => {
                self.layout_inline(containing_block, root_block, parent_height, inline_block)
            }
            TableRowNode(..) => self.layout_tablerow(containing_block, root_block, parent_height),
        }
    }

    fn layout_tablerow(
        &mut self,
        containing_block: &mut Dimensions,
        root_block: &Dimensions,
        parent_height: Option<f32>,
    ) {
        self.layout_inline(containing_block, root_block, parent_height, false);

        containing_block.used_height += self.dimensions.content.height;
        containing_block.used_width = 0.0;
    }

    /// Lay out a anonymous element and its descendants.
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/Visual_formatting_model>
    fn layout_anonymous(&mut self, containing_block: &Dimensions, root_block: &Dimensions) {
        let d = &mut self.dimensions;

        d.content.width = containing_block.content.width;
        d.content.x = containing_block.content.x;

        // Position the box below all the previous boxes in the container.
        d.content.y = containing_block.content.height + containing_block.content.y;

        // FIXME: if there are only inlince children, the height shouldn't be added. is this a problem here?

        // Recursively lay out the children of this box.
        self.layout_anonymous_children(root_block);
    }

    /// Lay out the block's children within its content area.
    ///
    /// Sets `self.dimensions.height` to the total content height.
    fn layout_anonymous_children(&mut self, root_block: &Dimensions) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(d, root_block, None);
            // Increment the height so each child is laid out below the previous one.
            d.content.height += child.dimensions.margin_box().height;
        }
    }

    /// Where a new inline child should go.
    pub fn get_inline_container(&mut self) -> &mut Self {
        match self.box_type {
            AnonymousBlock => self,
            TableRowNode(..) | InlineNode(..) | BlockNode(..) => {
                // If we've just generated an anonymous block box, keep using it.
                // Otherwise, create a new one.
                match self.children.last() {
                    Some(&Self {
                        box_type: AnonymousBlock,
                        ..
                    }) => {}
                    _ => self.children.push(Self::new(AnonymousBlock)),
                }
                self.children.last_mut().unwrap()
            }
        }
    }
}
