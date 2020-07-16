use crate::css::{Color, Unit, Value};
use crate::data_storage;
use crate::dom;
use crate::layout::{self, lbox, AnonymousBlock, BlockNode, InlineNode, Rect, TableRowNode};
use crate::stylednode::StyledNode;

#[derive(Debug)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
    /// foreground, text, rect, style, size, font-family
    Text(Color, String, Rect, Vec<String>, u16, String),
    Image(String, Rect),
    Gradient(Rect, u16, Vec<Color>),
}

pub type DisplayList = Vec<DisplayCommand>;

/// calls a layout with given dimensions and offsets
pub fn layout(style_root: StyledNode, width: f32, height: f32) -> lbox::LBox {
    let mut viewport: layout::Dimensions = layout::Dimensions::default();
    viewport.content.width = width;
    viewport.content.height = height;

    layout::layout_tree(style_root, viewport)
    // let layout_root =
    //println!("\n{:?}", layout_root);

    //layout_root
}

/// Converts a layout into a drawable `DisplayList`.
pub fn build_display_list(layout_root: &lbox::LBox) -> DisplayList {
    let mut list = Vec::new();
    render_layout_box(&mut list, layout_root);
    list
}

/// renders layout box and children
fn render_layout_box(list: &mut DisplayList, layout_box: &lbox::LBox) {
    let mut visible = true;
    if let Some(Value::Keyword(keyword)) = get_value(layout_box, "visibility") {
        // FIXME: `collapse` eigentlich andere funktion wenn in Tabelle
        if keyword == "hidden" || keyword == "collapse" {
            visible = false;
        }
    }

    if visible {
        render_background(list, layout_box);
        render_borders(list, layout_box);

        if let InlineNode(ref node, _) | BlockNode(ref node) = layout_box.box_type {
            if let dom::NodeType::Text(text) = &node.node.node_type {
                render_text(list, layout_box, text);
            } else if let dom::NodeType::Element(element) = &node.node.node_type {
                if element.tag_name == "img" {
                    if let Some(url) = element.src() {
                        render_image(list, layout_box, url);
                    }
                } else if element.tag_name == "video" {
                    if let Some(url) = element.get_attribute("poster") {
                        render_image(list, layout_box, url);
                    }
                }
            }
        }

        // FIXME: only draw point/number if `<li>` is inside a `<ul/ol>`
        if let InlineNode(node, _) | BlockNode(node) = &layout_box.box_type {
            if let dom::NodeType::Element(el) = &node.node.node_type {
                if el.tag_name == "li" {
                    let list_style = get_value(layout_box, "list-style-type")
                        .unwrap_or_else(|| Value::Keyword(String::from("disc")));
                    if let Value::Keyword(value) = list_style {
                        if value != "none" {
                            let color = match get_value(layout_box, "color") {
                                Some(Value::Color(color)) => color,
                                _ => Color {
                                    r: 0,
                                    g: 0,
                                    b: 0,
                                    a: 255,
                                },
                            };

                            let margin_box = layout_box.dimensions.margin_box();
                            list.push(DisplayCommand::SolidColor(
                                color,
                                Rect {
                                    x: margin_box.x - 12.0,
                                    y: margin_box.y + 9.0,
                                    width: 6.0,
                                    height: 6.0,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }

    layout_box
        .children
        .iter()
        .for_each(|child| render_layout_box(list, child));
}

/// adds display command for background
fn render_background(list: &mut DisplayList, layout_box: &lbox::LBox) {
    if let Some(Value::Color(color)) = get_value(layout_box, "background-color") {
        list.push(DisplayCommand::SolidColor(
            color,
            layout_box.dimensions.border_box(),
        ));
    }

    if let Some(Value::Url(url)) = get_value(layout_box, "background-image") {
        render_image(list, layout_box, &url);
    } else if let Some(Value::Gradient(direction, colors)) =
        get_value(layout_box, "background-image")
    {
        list.push(DisplayCommand::Gradient(
            layout_box.dimensions.border_box(),
            direction,
            colors,
        ));
    }
}

/// Adds display command for borders.
fn render_borders(list: &mut DisplayList, layout_box: &lbox::LBox) {
    let d = &layout_box.dimensions;
    let border_box = d.border_box();

    // Top border
    if d.border.top != 0.0 {
        let color = if let Some(Value::Color(color)) = get_value(layout_box, "border-top-color") {
            color
        } else {
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }
        };
        list.push(DisplayCommand::SolidColor(
            color,
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: border_box.width,
                height: d.border.top,
            },
        ));
    }

    // Right border
    if d.border.right != 0.0 {
        let color = if let Some(Value::Color(color)) = get_value(layout_box, "border-right-color") {
            color
        } else {
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }
        };
        list.push(DisplayCommand::SolidColor(
            color,
            Rect {
                x: border_box.x + border_box.width - d.border.right,
                y: border_box.y,
                width: d.border.right,
                height: border_box.height,
            },
        ));
    }

    // Bottom border
    if d.border.bottom != 0.0 {
        let color = if let Some(Value::Color(color)) = get_value(layout_box, "border-bottom-color")
        {
            color
        } else {
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }
        };
        list.push(DisplayCommand::SolidColor(
            color,
            Rect {
                x: border_box.x,
                y: border_box.y + border_box.height - d.border.bottom,
                width: border_box.width,
                height: d.border.bottom,
            },
        ));
    }

    // Left left
    if d.border.left != 0.0 {
        let color = if let Some(Value::Color(color)) = get_value(layout_box, "border-left-color") {
            color
        } else {
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            }
        };
        list.push(DisplayCommand::SolidColor(
            color,
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: d.border.left,
                height: border_box.height,
            },
        ));
    }
}

/// adds display command for text
fn render_text(list: &mut DisplayList, layout_box: &lbox::LBox, text: &[String]) {
    let color = match get_value(layout_box, "color") {
        Some(Value::Color(color)) => color,
        _ => Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
    };

    let mut styles = Vec::new();
    if let Some(Value::Keyword(style)) = get_value(layout_box, "text-decoration") {
        if let "underline" | "line-through" = &*style {
            styles.push(style);
        }
    }
    match get_value(layout_box, "font-weight") {
        Some(Value::Keyword(style)) => {
            if let "bold" | "bolder" = &*style {
                styles.push(String::from("bold"));
            }
        }
        Some(Value::Length(px, Unit::Zero)) => {
            if px > 500.0 {
                styles.push(String::from("bold"));
            }
        }
        _ => {}
    }
    if let Some(Value::Keyword(style)) = get_value(layout_box, "font-style") {
        if let "italic" = &*style {
            styles.push(style);
        }
    }

    let size = if let Some(value) = get_value(layout_box, "font-size") {
        // TODO: don't calculate here!
        // relativ to parent font size, parent width/height
        value.to_px(16.0, &layout::Dimensions::default()) as u16
    } else {
        16
    };

    let family = if let Some(Value::Keyword(keyword)) = get_value(layout_box, "font-family") {
        keyword
    } else {
        String::from("sans-serif")
    };

    let mut content = layout_box.dimensions.content;

    for line in text {
        list.push(DisplayCommand::Text(
            color.clone(),
            line.to_string(),
            content,
            styles.clone(),
            size,
            family.clone(),
        ));

        content.y += 20.0;
    }
}

/// adds display command for images
fn render_image(list: &mut DisplayList, layout_box: &lbox::LBox, url: &str) {
    // TODO: painting should not download. at any pointer earlier.
    // at best in/after layout, when it is known if the image is in the viewport

    if let Ok(path) = data_storage::download_cache_path(
        url,
        vec!["image/jpeg", "image/gif", "image/png", "image/webp"],
    ) {
        list.push(DisplayCommand::Image(path, layout_box.dimensions.content));
    } else if let layout::BoxType::InlineNode(node, _) = &layout_box.box_type {
        if let dom::NodeType::Element(element) = &node.node.node_type {
            if let Some(alt) = &element.get_attribute("alt") {
                render_text(list, layout_box, &[(*alt).to_string()]);
            }
        }
    }
}

/// Return the specified Value for CSS property `name`, or None if no Value was specified.
fn get_value(layout_box: &lbox::LBox, name: &str) -> Option<Value> {
    match layout_box.box_type {
        TableRowNode(ref style) | BlockNode(ref style) | InlineNode(ref style, _) => {
            style.value(name)
        }
        AnonymousBlock => None,
    }
}

/// Set off each element of a `DisplayList`
pub fn scroll(display_list: &mut DisplayList, y_offset: f32) {
    for item in display_list {
        match item {
            DisplayCommand::SolidColor(_, rect)
            | DisplayCommand::Text(_, _, rect, ..)
            | DisplayCommand::Image(_, rect)
            | DisplayCommand::Gradient(rect, ..) => {
                rect.y += y_offset;
            }
        }
    }
}
