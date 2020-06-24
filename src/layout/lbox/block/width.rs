use crate::css::Unit;
use crate::css::Value::{Keyword, Length};
use crate::layout::lbox::LBox;
use crate::layout::Dimensions;

impl LBox {
    /// Calculate the width of a block-level non-replaced element in normal flow.
    ///
    /// <http://www.w3.org/TR/CSS2/visudet.html#blockwidth>
    ///
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    pub fn calculate_block_width(
        &mut self,
        containing_block: &Dimensions,
        root_block: &Dimensions,
    ) {
        let style = self.get_style_node();
        // `width` has initial value `auto`.
        let auto = Keyword("auto".to_string());
        let mut width = style.lookup("width", &auto);
        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Unit::Px);
        let mut margin_left = style.lookup("margin-left", &zero);
        let mut margin_right = style.lookup("margin-right", &zero);

        let border_left = style
            .lookup("border-left-width", &zero)
            .to_px(0.0, root_block);
        let border_right = style
            .lookup("border-right-width", &zero)
            .to_px(0.0, root_block);

        let padding_left = style
            .lookup("padding-left", &zero)
            .to_px(containing_block.content.width, root_block);
        let padding_right = style
            .lookup("padding-right", &zero)
            .to_px(containing_block.content.width, root_block);

        let border_box = if let Some(Keyword(keyword)) = style.value("box-sizing") {
            keyword == "border-box"
        } else {
            false
        };

        // https://www.w3.org/TR/CSS2/visudet.html#min-max-widths
        let mut tentative_used_width = width.to_px(containing_block.content.width, root_block);

        // Checks `max-width`.
        if let Some(value) = style.value("max-width") {
            if let Length(..) = value {
                let max_width = value.to_px(containing_block.content.width, root_block);
                if tentative_used_width > max_width {
                    width = Length(max_width, Unit::Px);
                    tentative_used_width = max_width;
                }
            }
        }

        // Checks `min-width`.
        if let Some(value) = style.value("min-width") {
            let min_width = value.to_px(containing_block.content.width, root_block);
            if tentative_used_width < min_width {
                width = Length(min_width, Unit::Px);
                tentative_used_width = min_width;
            }
        }

        let minimum_width = margin_left.to_px(containing_block.content.width, root_block)
            + margin_right.to_px(containing_block.content.width, root_block)
            + if border_box {
                0.0
            } else {
                // Content box includes border and padding
                border_left + border_right + padding_left + padding_right
            }
            + tentative_used_width;

        // If width is not auto and the total is wider than the container, treat auto margins as 0.
        if width != auto && minimum_width > containing_block.content.width {
            if margin_left == auto {
                margin_left = zero.clone();
            }
            if margin_right == auto {
                margin_right = zero.clone();
            }
        }

        // Adjust used values so that the above sum equals `containing_block.width`.
        // Each arm of the `match` should increase the total width by exactly `underflow`,
        // and afterward all values should be absolute lengths in px.
        let underflow = containing_block.content.width - minimum_width;

        match (width == auto, margin_left == auto, margin_right == auto) {
            // If the values are overconstrained, calculate margin_right.
            (false, false, false) => {
                margin_right = Length(
                    margin_right.to_px(containing_block.content.width, root_block) + underflow,
                    Unit::Px,
                );
            }

            // If exactly one size is auto, its used value follows from the equality.
            (false, false, true) => {
                margin_right = Length(underflow, Unit::Px);
            }
            (false, true, false) => {
                margin_left = Length(underflow, Unit::Px);
            }

            // If width is set to auto, any other auto values become 0.
            (true, ..) => {
                if margin_left == auto {
                    margin_left = zero.clone();
                }
                if margin_right == auto {
                    margin_right = zero.clone();
                }

                if underflow >= 0.0 {
                    // Expand width to fill the underflow.
                    width = Length(underflow, Unit::Px);
                } else {
                    // Width can't be negative. Adjust the right margin instead.
                    width = zero;
                    margin_right = Length(
                        margin_right.to_px(containing_block.content.width, root_block) + underflow,
                        Unit::Px,
                    );
                }
            }

            // If margin-left and margin-right are both auto, their used values are equal.
            (false, true, true) => {
                margin_left = Length(underflow / 2.0, Unit::Px);
                margin_right = Length(underflow / 2.0, Unit::Px);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = width.to_px(containing_block.content.width, root_block)
            - if border_box {
                border_left + border_right + padding_left + padding_right
            } else {
                0.0
            };

        d.padding.left = padding_left;
        d.padding.right = padding_right;

        d.border.left = border_left;
        d.border.right = border_right;

        d.margin.left = margin_left.to_px(containing_block.content.width, root_block);
        d.margin.right = margin_right.to_px(containing_block.content.width, root_block);
    }
}

#[allow(clippy::float_cmp)]
#[cfg(test)]
mod width_test {
    use super::*;
    use crate::css::Value;
    use crate::dom;
    use crate::layout::{BoxType, Rect};
    use crate::stylednode::StyledNode;
    use std::collections::HashMap;

    // FIXME: not really testing anything
    #[test]
    fn empty() {
        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: HashMap::new(),
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_width(&Dimensions::default(), &Dimensions::default());

        let zero: f32 = 0.0;
        assert_eq!(lbox.dimensions.content.width, zero);
    }

    #[test]
    fn width() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(301.5, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_width(&Dimensions::default(), &Dimensions::default());

        assert_eq!(lbox.dimensions.content.width, 301.5);
    }

    #[test]
    fn max() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(301.5, Unit::Px));
        map.insert(String::from("max-width"), Value::Length(105.3, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_width(&Dimensions::default(), &Dimensions::default());

        assert_eq!(lbox.dimensions.content.width, 105.3);
    }

    #[test]
    fn percentage() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(25.0, Unit::Percentage));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        let mut contianing = Dimensions::default();
        contianing.content = Rect {
            x: 0.0,
            y: 0.0,
            width: 312.0,
            height: 0.0,
        };

        lbox.calculate_block_width(&contianing, &Dimensions::default());

        assert_eq!(lbox.dimensions.content.width, 78.0);
    }

    #[test]
    fn min() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(10.5, Unit::Px));
        map.insert(String::from("min-width"), Value::Length(105.3, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_width(&Dimensions::default(), &Dimensions::default());

        assert_eq!(lbox.dimensions.content.width, 105.3);
    }

    /// checks that min is dominant to max
    /// and min can be bigger than max
    #[test]
    fn max_and_min() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(301.5, Unit::Px));
        map.insert(String::from("min-width"), Value::Length(207.3, Unit::Px));
        map.insert(String::from("max-width"), Value::Length(105.3, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_width(&Dimensions::default(), &Dimensions::default());

        assert_eq!(lbox.dimensions.content.width, 207.3);
    }

    #[test]
    fn border_box() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(301.5, Unit::Px));
        map.insert(
            String::from("box-sizing"),
            Value::Keyword(String::from("border-box")),
        );
        map.insert(String::from("padding-left"), Value::Length(10.0, Unit::Px));
        map.insert(
            String::from("border-right-width"),
            Value::Length(7.0, Unit::Px),
        );

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        lbox.calculate_block_width(&Dimensions::default(), &Dimensions::default());

        assert_eq!(lbox.dimensions.content.width, 284.5);
    }

    /// Box should fill parent/contianing width
    #[test]
    fn auto() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Keyword(String::from("auto")));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        let mut containing = Dimensions::default();
        containing.content.width = 120.3;

        lbox.calculate_block_width(&containing, &Dimensions::default());

        assert_eq!(lbox.dimensions.content.width, 120.3);
    }

    /// left and right margin takes half of remaining space
    #[test]
    fn margin_auto() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(10.0, Unit::Px));
        map.insert(
            String::from("margin-right"),
            Value::Keyword(String::from("auto")),
        );
        map.insert(
            String::from("margin-left"),
            Value::Keyword(String::from("auto")),
        );

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        let mut containing = Dimensions::default();
        containing.content.width = 120.3;

        lbox.calculate_block_width(&containing, &Dimensions::default());

        assert_eq!(lbox.dimensions.margin.right, 55.15);
        assert_eq!(lbox.dimensions.margin.left, 55.15);
    }

    /// right margin takes all of remaining space
    #[test]
    fn margin_right_auto() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(10.0, Unit::Px));
        map.insert(
            String::from("margin-right"),
            Value::Keyword(String::from("auto")),
        );

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        let mut containing = Dimensions::default();
        containing.content.width = 120.3;

        lbox.calculate_block_width(&containing, &Dimensions::default());

        assert_eq!(lbox.dimensions.margin.right, 110.3);
        assert_eq!(lbox.dimensions.margin.left, 0.0);
    }

    /// right margin takes all of remaining space although set explicitly
    #[test]
    fn margin() {
        let mut map = HashMap::new();
        map.insert(String::from("width"), Value::Length(10.0, Unit::Px));
        map.insert(String::from("margin-right"), Value::Length(7.0, Unit::Px));
        map.insert(String::from("margin-left"), Value::Length(7.0, Unit::Px));

        let mut lbox = LBox::new(BoxType::BlockNode(StyledNode {
            children: Vec::new(),
            specified_values: map,
            node: dom::Node::text(String::new()),
        }));

        let mut containing = Dimensions::default();
        containing.content.width = 120.3;

        lbox.calculate_block_width(&containing, &Dimensions::default());

        assert_eq!(lbox.dimensions.margin.right, 103.3);
    }
}
