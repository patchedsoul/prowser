//! This module contains the lbox layouting code for __block__ nodes.

mod width;

use crate::css::Unit;
use crate::css::Value::{Keyword, Length};
use crate::layout::lbox::{LBox, Position};
use crate::layout::Dimensions;

impl LBox {
    /// Lay out a block-level element and its descendants.
    pub fn layout_block(
        &mut self,
        containing_block: &mut Dimensions,
        root_block: &Dimensions,
        parent_height: Option<f32>,
    ) {
        // Child width can depend on parent width, so we need to calculate this box's width before
        // laying out its children.
        self.calculate_block_width(containing_block, root_block);

        // Determine where the box is located within its container.
        self.calculate_block_position(containing_block, root_block);

        // Recursively lay out the children of this box.
        self.layout_block_children(root_block, parent_height);

        // Parent height can depend on child height, so `calculate_height` must be called after the
        // children are laid out.
        self.calculate_block_height(root_block, parent_height);
    }

    /// Finish calculating the block's edge sizes, and position it within its containing block.
    ///
    /// <http://www.w3.org/TR/CSS2/visudet.html#normal-block>
    ///
    /// Sets the vertical margin/padding/border dimensions, and the `x`, `y` values.
    fn calculate_block_position(&mut self, containing_block: &Dimensions, root_block: &Dimensions) {
        let style = self.get_style_node().clone();
        let d = &mut self.dimensions;
        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Unit::Px);

        // If margin-top or margin-bottom is `auto`, the used value is zero.
        d.margin.top = style
            .lookup("margin-top", &zero)
            .to_px(containing_block.content.width, root_block);
        d.margin.bottom = style
            .lookup("margin-bottom", &zero)
            .to_px(containing_block.content.width, root_block);

        d.border.top = style
            .lookup("border-top-width", &zero)
            .to_px(0.0, root_block);
        d.border.bottom = style
            .lookup("border-bottom-width", &zero)
            .to_px(0.0, root_block);

        d.padding.top = style
            .lookup("padding-top", &zero)
            .to_px(containing_block.content.width, root_block);
        d.padding.bottom = style
            .lookup("padding-bottom", &zero)
            .to_px(containing_block.content.width, root_block);

        let position = style
            .value("position")
            .unwrap_or_else(|| Keyword(String::from("static")));

        if let Keyword(keyword) = position {
            match &*keyword {
                "absolute" | "fixed" => {
                    // FIXME: which value if unset? its not 0.
                    // FIXME: can it be percentage?
                    d.content.x = match style.value("left") {
                        Some(left) => {
                            left.to_px(0.0, root_block)
                                + d.margin.left
                                + d.border.left
                                + d.padding.left
                        }
                        None => match style.value("right") {
                            Some(right) => {
                                root_block.content.width
                                    - right.to_px(0.0, root_block)
                                    - d.margin.right
                                    - d.border.right
                                    - d.padding.right
                                    - d.content.width
                            }
                            None => 0.0,
                        },
                    };

                    d.content.y = match style.value("top") {
                        Some(top) => {
                            top.to_px(0.0, root_block)
                                + containing_block.content.y
                                + d.margin.top
                                + d.border.top
                                + d.padding.top
                        }
                        None => match style.value("bottom") {
                            Some(bottom) => {
                                root_block.content.height
                                    - bottom.to_px(0.0, root_block)
                                    - d.margin.bottom
                                    - d.border.bottom
                                    - d.padding.bottom
                            }
                            None => 0.0,
                        },
                    };

                    self.position = Position::Fixed;
                }
                _ => {
                    d.content.x =
                        containing_block.content.x + d.margin.left + d.border.left + d.padding.left;

                    // Position the box below all the previous boxes in the container.
                    d.content.y = containing_block.content.height
                        + containing_block.content.y
                        + d.margin.top
                        + d.border.top
                        + d.padding.top;
                }
            }
        }
    }

    /// Lay out the block's children within its content area.
    ///
    /// Sets `self.dimensions.height` to the total content height.
    fn layout_block_children(&mut self, root_block: &Dimensions, parent_height: Option<f32>) {
        let mut height = None;
        if let Some(length) = self.get_style_node().value("height") {
            if let Length(_, Unit::Percentage) = length {
                if let Some(parent_height) = parent_height {
                    height = Some(length.to_px(parent_height, root_block));
                }
            } else {
                height = Some(length.to_px(0.0, root_block));
            }
        }

        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(d, root_block, height);
            if let Position::Fixed = child.position {
                // fixed positioned elements don't take space in the normal flow. Don't reserve space for them.
            } else {
                // Increment the height so each child is laid out below the previous one.
                d.content.height += child.dimensions.margin_box().height;
            }
        }
    }

    /// Height of a block-level non-replaced element in normal flow with overflow visible.
    fn calculate_block_height(&mut self, root_block: &Dimensions, parent_height: Option<f32>) {
        let style = self.get_style_node().clone();
        let d = &mut self.dimensions;

        let border_box = if let Some(Keyword(keyword)) = style.value("box-sizing") {
            keyword == "border-box"
        } else {
            false
        };

        // If the height is set to an explicit length, use that exact length.
        // Otherwise, just keep the value set by `layout_block_children`.
        if let Some(length) = style.value("height") {
            match length {
                Length(_, Unit::Percentage) => {
                    /* FIXME: height: 100% will break scrolling. Overflow needs to be handled in some way
                    height calculation muss wirklich height zurueck geben.
                    Aber dadruch wird die gesamte page size auch zu klein gesetzt.*/
                    if let Some(parent_height) = parent_height {
                        d.content.height = length.to_px(parent_height, root_block)
                            - if border_box {
                                // Border box doesn't includes border and padding
                                d.border.top + d.border.bottom + d.padding.top + d.padding.bottom
                            } else {
                                0.0
                            };
                    }
                }
                Length(..) => {
                    d.content.height = length.to_px(0.0, root_block)
                        - if border_box {
                            // Border box doesn't includes border and padding
                            d.border.top + d.border.bottom + d.padding.top + d.padding.bottom
                        } else {
                            0.0
                        };
                }
                _ => {}
            }
        }
        if let Some(length) = style.value("max-height") {
            let mut max_height = None;

            match length {
                Length(_, Unit::Percentage) => {
                    // The height of the containing block must be specified explicitly.
                    if let Some(parent_height) = parent_height {
                        max_height = Some(length.to_px(parent_height, root_block));
                    }
                }
                Length(..) => {
                    max_height = Some(length.to_px(0.0, root_block));
                }
                _ => {}
            }

            if let Some(max) = max_height {
                if d.content.height > max {
                    d.content.height = max;
                }
            }
        }
        if let Some(length) = style.value("min-height") {
            let min_height = if let Some(parent_height) = parent_height {
                length.to_px(parent_height, root_block)
            } else {
                length.to_px(0.0, root_block)
            } - if border_box {
                // Border box doesn't includes border and padding
                d.border.top + d.border.bottom + d.padding.top + d.padding.bottom
            } else {
                0.0
            };

            if d.content.height < min_height {
                d.content.height = min_height;
            }
        }
    }
}

#[allow(clippy::float_cmp)]
#[cfg(test)]
mod block_test {
    use super::*;
    use crate::css::Value;
    use crate::dom;
    use crate::layout::BoxType;
    use crate::stylednode::StyledNode;
    use std::collections::HashMap;

    // FIXME: not really testing anything
    #[test]
    fn empty_height() {
        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: HashMap::new(),
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_height(&Dimensions::default(), Some(0.0));

        let zero: f32 = 0.0;
        assert_eq!(lbox.dimensions.content.height, zero);
    }

    #[test]
    fn height() {
        let mut map = HashMap::new();
        map.insert(String::from("height"), Value::Length(301.5, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_height(&Dimensions::default(), Some(0.0));

        assert_eq!(lbox.dimensions.content.height, 301.5);
    }

    #[test]
    fn min_height() {
        let mut map = HashMap::new();
        map.insert(String::from("height"), Value::Length(301.5, Unit::Px));
        map.insert(String::from("min-height"), Value::Length(433.5, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_height(&Dimensions::default(), Some(0.0));

        assert_eq!(lbox.dimensions.content.height, 433.5);
    }

    #[test]
    fn min_height_not_used() {
        let mut map = HashMap::new();
        map.insert(String::from("height"), Value::Length(301.5, Unit::Px));
        map.insert(String::from("min-height"), Value::Length(100.5, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_height(&Dimensions::default(), Some(0.0));

        assert_eq!(lbox.dimensions.content.height, 301.5);
    }

    #[test]
    fn max_height() {
        let mut map = HashMap::new();
        map.insert(String::from("height"), Value::Length(301.5, Unit::Px));
        map.insert(String::from("max-height"), Value::Length(100.5, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_height(&Dimensions::default(), Some(0.0));

        assert_eq!(lbox.dimensions.content.height, 100.5);
    }

    #[test]
    fn height_border_box() {
        let mut map = HashMap::new();
        map.insert(String::from("height"), Value::Length(301.5, Unit::Px));
        map.insert(
            String::from("box-sizing"),
            Value::Keyword(String::from("border-box")),
        );

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.dimensions.padding.top = 10.0;
        lbox.dimensions.border.bottom = 7.0;

        lbox.calculate_block_height(&Dimensions::default(), Some(0.0));

        assert_eq!(lbox.dimensions.content.height, 284.5);
    }

    /// padding percentage is based on the parent element's width
    #[test]
    fn padding_percentage() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(40.0, Unit::Px));
        map.insert(
            String::from("padding-top"),
            Value::Length(50.0, Unit::Percentage),
        );

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        let mut containing = Dimensions::default();
        containing.content.width = 450.0;

        lbox.calculate_block_position(&containing, &Dimensions::default());

        assert_eq!(lbox.dimensions.padding.top, 225.0);
    }

    #[test]
    fn position_y() {
        let mut map = HashMap::new();
        map.insert(String::from("padding-top"), Value::Length(301.5, Unit::Px));
        map.insert(String::from("margin-top"), Value::Length(2.5, Unit::Px));
        map.insert(
            String::from("border-top-width"),
            Value::Length(5.0, Unit::Px),
        );

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_position(&Dimensions::default(), &Dimensions::default());

        assert_eq!(lbox.dimensions.content.y, 309.0);
    }

    #[test]
    fn position_y_containing() {
        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: HashMap::new(),
            node: dom::Node::text(String::new()),
        }));

        let mut containing = Dimensions::default();
        containing.content.y = 120.0;
        containing.content.height = 3.0;

        lbox.calculate_block_position(&containing, &Dimensions::default());

        assert_eq!(lbox.dimensions.content.y, 123.0);
    }

    #[test]
    fn position_x() {
        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: HashMap::new(),
            node: dom::Node::text(String::new()),
        }));

        lbox.dimensions.margin.left = 2.5;
        lbox.dimensions.padding.left = 301.5;
        lbox.dimensions.border.left = 5.0;

        lbox.calculate_block_position(&Dimensions::default(), &Dimensions::default());

        assert_eq!(lbox.dimensions.content.x, 309.0);
    }

    #[test]
    fn position_x_containing() {
        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: HashMap::new(),
            node: dom::Node::text(String::new()),
        }));

        let mut containing = Dimensions::default();
        containing.content.x = 120.0;

        lbox.calculate_block_position(&containing, &Dimensions::default());

        assert_eq!(lbox.dimensions.content.x, 120.0);
    }

    #[test]
    fn children_height() {
        let mut map_child1 = HashMap::new();
        let mut map_child2 = HashMap::new();
        map_child1.insert(String::from("height"), Value::Length(124.5, Unit::Px));
        map_child2.insert(String::from("height"), Value::Length(245.3, Unit::Px));

        let mut lbox_parent = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: HashMap::new(),
            node: dom::Node::text(String::new()),
        }));

        lbox_parent.children = vec![
            LBox::new(BoxType::BlockNode(StyledNode {
                children: Vec::new(),
                specified_values: map_child1,
                node: dom::Node::text(String::new()),
            })),
            LBox::new(BoxType::BlockNode(StyledNode {
                children: Vec::new(),
                specified_values: map_child2,
                node: dom::Node::text(String::new()),
            })),
        ];

        lbox_parent.layout_block_children(&Dimensions::default(), None);

        assert_eq!(lbox_parent.dimensions.content.height, 369.8);
    }

    #[test]
    fn position_fixed() {
        let mut map = HashMap::new();
        map.insert(
            String::from("position"),
            Value::Keyword(String::from("fixed")),
        );
        map.insert(String::from("top"), Value::Length(5.2, Unit::Px));
        map.insert(String::from("right"), Value::Length(2.5, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        let mut containing = Dimensions::default();
        containing.content.width = 450.0;

        lbox.calculate_block_position(&Dimensions::default(), &containing);

        assert_eq!(lbox.dimensions.content.y, 5.2);
        assert_eq!(lbox.dimensions.content.x, 447.5);
    }
}
