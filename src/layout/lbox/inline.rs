//! This module contains the lbox layouting code for __inline(-block)__ nodes.

use crate::css::Unit;
use crate::css::Value::{Keyword, Length};
use crate::dom;
use crate::layout::lbox::LBox;
use crate::layout::{Dimensions, InlineNode};

impl LBox {
    /// Lay out a inline-level element and its descendants.
    ///
    /// <https://www.w3.org/TR/CSS2/visuren.html#inline-formatting>
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Flow_Layout/Block_and_Inline_Layout_in_Normal_Flow>
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Flow_Layout/In_Flow_and_Out_of_Flow>
    pub fn layout_inline(
        &mut self,
        containing_block: &mut Dimensions,
        root_block: &Dimensions,
        parent_height: Option<f32>,
        inline_block: bool,
    ) {
        self.calculate_inline_width(containing_block, root_block, inline_block);

        // Determine where the box is located within its container.
        self.calculate_inline_position(containing_block, root_block);

        // Recursively lay out the children of this box.
        self.layout_inline_children(containing_block, root_block, parent_height);

        // Check if height needs to be set, if it is a inline-block
        if inline_block {
            self.calculate_inline_height(root_block);
        }
    }

    /// Lay out the block's children within its content area.
    ///
    /// Sets `self.dimensions.height/width` to the total content height/width.
    fn layout_inline_children(
        &mut self,
        containing_block: &mut Dimensions,
        root_block: &Dimensions,
        parent_height: Option<f32>,
    ) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(d, root_block, parent_height);

            let child_marginbox = child.dimensions.margin_box();

            if child_marginbox.width + d.content.width > containing_block.content.width {
                child.dimensions.content.y += d.content.height;
                child.dimensions.content.x -= d.content.width;
                d.content.height += child_marginbox.height;
                containing_block.used_width = 0.0;
                if child_marginbox.width > d.content.width {
                    d.content.width = child_marginbox.width;
                }
            } else {
                // Increment the width/height.
                let child_height = child_marginbox.height;
                // only add height if child is taller than other children
                // FIXME: only apply to current line. on second line d.content.height is much likly bigger than child
                if child_height > d.content.height {
                    d.content.height = child_height;
                }
                d.content.width += child_marginbox.width;
            }
        }

        // FIXME: two tests dont pass with this, but without https://limpet.net/mbrubeck/2014/08/11/toy-layout-engine-2.html doesnt load
        // Position the box next to all the previous boxes in the container or break lines.
        /* if containing_block.content.width < d.content.width + containing_block.used_width {
            d.content.y += 19.0;
            d.content.x -= containing_block.used_width;
            containing_block.used_height += d.content.height;
            containing_block.used_width = 0.0;
        } */

        // FIXME: seems like a hack. Why is it needed?
        // if only one child, set same x/y to overwrite possible false values
        if self.children.len() == 1 {
            if let InlineNode(..) = self.children[0].box_type {
                self.children[0].dimensions.content.x = d.content.x;
                self.children[0].dimensions.content.y = d.content.y;
            }
        }

        containing_block.used_width += d.content.width;
    }

    /// Calculates `height` in respect of `min`/`max-height`
    fn calculate_inline_height(&mut self, root_block: &Dimensions) {
        let style = self.get_style_node().clone();
        let d = &mut self.dimensions;

        let border_box = if let Some(Keyword(keyword)) = style.value("box-sizing") {
            keyword == "border-box"
        } else {
            false
        };

        // If the height is set to an explicit length, use that exact length.
        if let Some(length) = style.value("height") {
            match length {
                Length(_, ref unit) if unit != &Unit::Percentage => {
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
            match length {
                Length(_, ref unit) if unit != &Unit::Percentage => {
                    let max_height = length.to_px(0.0, root_block);
                    if d.content.height > max_height {
                        d.content.height = max_height;
                    }
                }
                _ => {}
            }
        }
        if let Some(length) = style.value("min-height") {
            match length {
                Length(_, ref unit) if unit != &Unit::Percentage => {
                    let min_height = length.to_px(0.0, root_block)
                        - if border_box {
                            // Border box doesn't includes border and padding
                            d.border.top + d.border.bottom + d.padding.top + d.padding.bottom
                        } else {
                            0.0
                        };
                    if d.content.height < min_height {
                        d.content.height = min_height;
                    }
                }
                _ => {}
            }
        }
    }

    /// Calculate the width of a inline-level non-replaced element in normal flow.
    ///
    /// <https://www.w3.org/TR/CSS2/visudet.html#inline-width>
    ///
    /// Sets the horizontal margin/padding/border dimensions, and the `width`.
    fn calculate_inline_width(
        &mut self,
        containing_block: &mut Dimensions,
        root_block: &Dimensions,
        inline_block: bool,
    ) {
        let style = self.get_style_node().clone();
        // margin, border, and padding have initial value 0.
        let zero = Length(0.0, Unit::Px);

        let margin_left = style
            .lookup("margin-left", &zero)
            .to_px(containing_block.content.width, root_block);
        let margin_right = style
            .lookup("margin-right", &zero)
            .to_px(containing_block.content.width, root_block);

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

        let d = &mut self.dimensions;

        if let InlineNode(ref mut node, _) = self.box_type {
            if let dom::NodeType::Text(ref mut text) = &mut node.node.node_type {
                let size = style.lookup("font-size", &Length(16.0, Unit::Px));
                // relativ to parent font size
                let font_size = size.to_px(16.0, root_block);

                // size_new in pixel
                /*style
                .specified_values
                .insert("name".to_string(), Length(2.9, Px));*/

                /*
                TODO: remember font size
                width canvas, calculate correct size
                */

                // remember height and width
                // FIXME: calculate font size depending on font-art/mono/css/...

                let mut line = 0;
                let mut widest_line = 0.0;
                let max_width = containing_block.content.width - containing_block.used_width;
                let all = text[0].clone();
                let mut last = 0;

                loop {
                    let current_width = text[line].len() as f32 * 0.513 * font_size;

                    // line to wide
                    // 0.0 check is needed if text is child of inline element and not block
                    if current_width > max_width && max_width != 0.0 {
                        let mut end = (max_width / 0.513 / font_size) as usize;

                        if !all.is_char_boundary(last + end) {
                            end += 1;
                            if !all.is_char_boundary(last + end) {
                                end += 1;
                            }
                        }

                        // end is not allowed exceed text length
                        if last + end > all.len() {
                            text[line] = all[last..all.len()].to_string();
                            break;
                        } else {
                            text[line] = all[last..last + end].to_string();
                            last += end;
                        }

                        text.push(all[last..].to_string());

                        // remember longest line
                        if current_width > widest_line {
                            widest_line = current_width;
                        }

                        line += 1;
                    } else {
                        // if one line is enough
                        if widest_line == 0.0 {
                            widest_line = current_width;
                        }
                        // line not to wide anymore
                        break;
                    }
                }

                d.content.width = widest_line;
                d.content.height = (font_size + 4.0) * text.len() as f32;
            } else if let dom::NodeType::Element(element) = &node.node.node_type {
                if element.tag_name == "img"
                    || element.tag_name == "video"
                    || element.tag_name == "object"
                    || element.tag_name == "embed"
                    || element.tag_name == "canvas"
                    || element.tag_name == "iframe"
                {
                    if let Some(width) = style.value("width") {
                        d.content.width = width.to_px(16.0, root_block);
                    } else if let Some(width) = style.attribute("width") {
                        // The intrinsic width of the image in pixels. Must be an integer without a unit.
                        // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img#attr-width
                        // https://html.spec.whatwg.org/multipage/embedded-content-other.html#attr-dim-width
                        if let Ok(set_width) = width.parse::<f32>() {
                            d.content.width = set_width;
                        }
                    } else {
                        // FIXME: calculate image dimensions correctly, like respecting aspect ratio if no css/attribute dimensions set
                        d.content.width = 500.0;
                    }

                    if let Some(height) = style.value("height") {
                        d.content.height = height.to_px(16.0, root_block);
                    } else if let Some(height) = style.attribute("height") {
                        if let Ok(set_height) = height.parse::<f32>() {
                            d.content.height = set_height;
                        }
                    } else {
                        d.content.height = 300.0;
                    }
                } else if element.tag_name == "br" {
                    // don't take dimensions. set `used_width` back and increase `used_height`
                    // FIXME: use font-height value
                    containing_block.used_height += 19.0;
                    containing_block.used_width = 0.0;
                }
            }
        } else {
            unreachable!("in inline layout method must always be an inline node");
        }

        if inline_block {
            let border_box = if let Some(Keyword(keyword)) = style.value("box-sizing") {
                keyword == "border-box"
            } else {
                false
            };

            // If the width is set to an explicit length, use that exact length.
            if let Some(width) = style.value("width") {
                if let Length(..) = width {
                    d.content.width = width.to_px(containing_block.content.width, root_block)
                        - if border_box {
                            border_left + border_right + padding_left + padding_right
                        } else {
                            0.0
                        };
                }
            }
            // Checks `max-width`
            if let Some(value) = style.value("max-width") {
                if let Length(..) = value {
                    let max_width = value.to_px(containing_block.content.width, root_block);
                    if d.content.width > max_width {
                        d.content.width = max_width
                            - if border_box {
                                border_left + border_right + padding_left + padding_right
                            } else {
                                0.0
                            };
                    }
                }
            }
            // Checks `min-width`.
            if let Some(value) = style.value("min-width") {
                let min_width = value.to_px(containing_block.content.width, root_block);
                if d.content.width < min_width {
                    d.content.width = min_width
                        - if border_box {
                            border_left + border_right + padding_left + padding_right
                        } else {
                            0.0
                        };
                }
            }
        }

        d.padding.left = padding_left;
        d.padding.right = padding_right;

        d.border.left = border_left;
        d.border.right = border_right;

        d.margin.left = margin_left;
        d.margin.right = margin_right;
    }

    /// Finish calculating the block's edge sizes, and position it within its containing block.
    ///
    /// <http://www.w3.org/TR/CSS2/visudet.html#normal-block>
    ///
    /// Sets the vertical margin/padding/border dimensions.
    fn calculate_inline_position(
        &mut self,
        containing_block: &Dimensions,
        root_block: &Dimensions,
    ) {
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
            .to_px(d.content.width + d.margin.left + d.margin.right, root_block);
        d.padding.bottom = style
            .lookup("padding-bottom", &zero)
            .to_px(d.content.width + d.margin.left + d.margin.right, root_block);

        d.content.x = containing_block.used_width
            + containing_block.content.x
            + d.margin.left
            + d.border.left
            + d.padding.left;

        d.content.y = containing_block.used_height
            + containing_block.content.y
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }
}

#[allow(clippy::float_cmp)]
#[cfg(test)]
mod inline_test {
    use super::*;
    use crate::css::Value;
    use crate::dom;
    use crate::layout::BoxType;
    use crate::stylednode::StyledNode;
    use std::collections::HashMap;

    #[test]
    fn two_next() {
        let mut lbox = LBox::new(BoxType::InlineNode(
            StyledNode {
                children: Vec::new(),
                specified_values: HashMap::new(),
                node: dom::Node::elem(String::from("span"), HashMap::new(), Vec::new()),
            },
            false,
        ));

        let mut map1 = HashMap::new();
        map1.insert(String::from("width"), Value::Length(112.5, Unit::Px));
        map1.insert(String::from("height"), Value::Length(200.0, Unit::Px));

        let mut map2 = HashMap::new();
        map2.insert(String::from("width"), Value::Length(42.5, Unit::Px));
        map2.insert(String::from("height"), Value::Length(25.0, Unit::Px));

        lbox.children = vec![
            LBox::new(BoxType::InlineNode(
                StyledNode {
                    children: Vec::new(),
                    specified_values: map1,
                    node: dom::Node::elem(String::from("img"), HashMap::new(), Vec::new()),
                },
                false,
            )),
            LBox::new(BoxType::InlineNode(
                StyledNode {
                    children: Vec::new(),
                    specified_values: map2,
                    node: dom::Node::elem(String::from("img"), HashMap::new(), Vec::new()),
                },
                false,
            )),
        ];

        let mut parent = Dimensions::default();
        parent.content.width = 1000.0;

        lbox.layout_inline(&mut parent, &Dimensions::default(), None, false);

        assert_eq!(lbox.dimensions.content.width, 155.0);
        assert_eq!(lbox.dimensions.content.height, 200.0);
        // FIXME: why is it 19?
        assert_eq!(lbox.children[0].dimensions.content.y, 0.0);
        assert_eq!(lbox.children[0].dimensions.content.x, 0.0);
        assert_eq!(lbox.children[1].dimensions.content.y, 0.0);
        assert_eq!(lbox.children[1].dimensions.content.x, 112.5);
    }

    #[test]
    fn two_over() {
        let mut lbox = LBox::new(BoxType::InlineNode(
            StyledNode {
                children: Vec::new(),
                specified_values: HashMap::new(),
                node: dom::Node::elem(String::from("span"), HashMap::new(), Vec::new()),
            },
            false,
        ));

        let mut map1 = HashMap::new();
        map1.insert(String::from("width"), Value::Length(112.5, Unit::Px));
        map1.insert(String::from("height"), Value::Length(200.0, Unit::Px));

        let mut map2 = HashMap::new();
        map2.insert(String::from("width"), Value::Length(42.5, Unit::Px));
        map2.insert(String::from("height"), Value::Length(25.0, Unit::Px));

        lbox.children = vec![
            LBox::new(BoxType::InlineNode(
                StyledNode {
                    children: Vec::new(),
                    specified_values: map1,
                    node: dom::Node::elem(String::from("img"), HashMap::new(), Vec::new()),
                },
                false,
            )),
            LBox::new(BoxType::InlineNode(
                StyledNode {
                    children: Vec::new(),
                    specified_values: map2,
                    node: dom::Node::elem(String::from("img"), HashMap::new(), Vec::new()),
                },
                false,
            )),
        ];

        let mut parent = Dimensions::default();
        parent.content.width = 120.0;

        lbox.layout_inline(&mut parent, &Dimensions::default(), None, false);

        assert_eq!(lbox.dimensions.content.width, 112.5);
        assert_eq!(lbox.dimensions.content.height, 225.0);
        assert_eq!(lbox.children[0].dimensions.content.y, 0.0);
        assert_eq!(lbox.children[0].dimensions.content.x, 0.0);
        assert_eq!(lbox.children[1].dimensions.content.y, 200.0);
        assert_eq!(lbox.children[1].dimensions.content.x, 0.0);
    }
}
