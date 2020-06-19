pub mod parser;

use crate::css::{self, Value};
use crate::layout;

/// [lvl4](https://drafts.csswg.org/mediaqueries/#media-types)
#[derive(Debug)]
struct MediaQuery {
    media_type: String,
    media_features: Vec<(MediaFeature, char)>,
    not: bool,
}

#[derive(Debug)]
enum MediaFeature {
    /// Declatiaion, not (inverted)
    Declaration(css::Declaration, bool),
    // Feature without value, `color`
    Name(String, bool),
}

/// Checks if a query matches
// FIXME: () syntax anfangen, dann ist jeder Teil entweder and oder or.
fn matches(query: &MediaQuery, dimensions: (u32, u32)) -> bool {
    let mut matching = !(!query.media_type.is_empty()
        && query.media_type != "screen"
        && query.media_type != "all");

    if matching {
        for (feature, _) in &query.media_features {
            if !feature_matches(feature, dimensions) {
                matching = false;
            }
        }
    }

    /* not working correctly. () are connected with `and` or `or`.
    if matching {
        let mut feature_matching = false;
        for (feature, combinator) in &query.features {
            if combinator == &'&' && !feature_matches(feature, dimensions) {
                feature_matching = false;
                break;
            } else if (combinator == &'|' || combinator == &'-')
                && feature_matches(feature, dimensions)
            {
                feature_matching = true;
            }
        }
        matching = feature_matching;
    }
    */

    // matching inverted if query is not prefixed
    matching ^ query.not
}

/// Checks if a feature matches
fn feature_matches(feature: &MediaFeature, dimensions: (u32, u32)) -> bool {
    match feature {
        MediaFeature::Name(name, not) => (name == "color") ^ not, // || name == "hover"
        MediaFeature::Declaration(declaration, not) => {
            declaration_matches(declaration, dimensions) ^ not
        }
    }
}

/// Checks if a declaration matches
fn declaration_matches(declaration: &css::Declaration, dimensions: (u32, u32)) -> bool {
    /* TODO:
    take user config into consideration:
    - prefers-reduced-motion
    - prefers-color-scheme
    */

    match &*declaration.name {
        "aspect-ratio" => {
            if let Value::Ratio(x, y) = declaration.value {
                #[allow(clippy::float_cmp)]
                return x as f32 / y as f32 == dimensions.0 as f32 / dimensions.1 as f32;
            }
            false
        }
        "min-aspect-ratio" => {
            if let Value::Ratio(x, y) = declaration.value {
                return (x as f32 / y as f32) < dimensions.0 as f32 / dimensions.1 as f32;
            }
            false
        }
        "max-aspect-ratio" => {
            if let Value::Ratio(x, y) = declaration.value {
                return x as f32 / y as f32 > dimensions.0 as f32 / dimensions.1 as f32;
            }
            false
        }
        "width" => {
            dimensions.0 == declaration.value.to_px(0.0, &layout::Dimensions::default()) as u32
        }
        "max-width" => {
            dimensions.0 <= declaration.value.to_px(0.0, &layout::Dimensions::default()) as u32
        }
        "min-width" => {
            dimensions.0 >= declaration.value.to_px(0.0, &layout::Dimensions::default()) as u32
        }
        "height" => {
            dimensions.1 == declaration.value.to_px(0.0, &layout::Dimensions::default()) as u32
        }
        "max-height" => {
            dimensions.1 <= declaration.value.to_px(0.0, &layout::Dimensions::default()) as u32
        }
        "min-height" => {
            dimensions.1 >= declaration.value.to_px(0.0, &layout::Dimensions::default()) as u32
        }
        "orientation" => {
            if let Value::Keyword(keyword) = &declaration.value {
                (keyword == "landscape" || keyword == "portrait") && dimensions.0 > dimensions.1
            } else {
                false
            }
        }
        "update" => {
            if let Value::Keyword(keyword) = &declaration.value {
                if keyword == "fast" {
                    return true;
                }
            }
            false
        }
        "display-mode" => {
            if let Value::Keyword(keyword) = &declaration.value {
                // FIXME: fullscreen wenn in fullscreen [F11] https://developer.mozilla.org/en-US/docs/Web/CSS/@media/display-mode
                if keyword == "browser" {
                    return true;
                }
            }
            false
        }
        "inverted-colors" | "scripting" => {
            if let Value::Keyword(keyword) = &declaration.value {
                if keyword == "none" {
                    return true;
                }
            }
            false
        }
        "pointer" | "any-pointer" => {
            if let Value::Keyword(keyword) = &declaration.value {
                if keyword == "fine" {
                    return true;
                }
            }
            false
        }
        "hover" => {
            if let Value::Keyword(keyword) = &declaration.value {
                if keyword == "hover" {
                    return true;
                }
            }
            false
        }
        "light-level" => {
            if let Value::Keyword(keyword) = &declaration.value {
                if keyword == "normal" {
                    return true;
                }
            }
            false
        }
        "prefers-reduced-motion" => {
            if let Value::Keyword(keyword) = &declaration.value {
                if keyword == "no-preference" {
                    return true;
                }
            }
            false
        }
        "prefers-color-scheme" => {
            if let Value::Keyword(keyword) = &declaration.value {
                if keyword == "dark" {
                    return true;
                }
            }
            false
        }
        "min-monochrome" | "max-monochrome" => {
            0 == declaration.value.to_px(0.0, &layout::Dimensions::default()) as u32
        }
        "overflow-block" | "overflow-inline" => {
            if let Value::Keyword(keyword) = &declaration.value {
                if keyword == "scroll" {
                    return true;
                }
            }
            false
        }
        "grid" => {
            if let css::Value::Number(0) = &declaration.value {
                return true;
            }
            false
        }
        "resolution" => {
            // FIXME: assuming 96dpi
            if let css::Value::Number(96) = &declaration.value {
                return true;
            }
            false
        }
        "min-resolution" => {
            // FIXME: assuming 96dpi
            if let css::Value::Number(dpi) = &declaration.value {
                // is display dpi bigger or equal to required
                return &96 >= dpi;
            }
            false
        }
        "max-resolution" => {
            // FIXME: assuming 96dpi
            if let css::Value::Number(dpi) = &declaration.value {
                // is display dpi smaller or equal to required
                return &96 <= dpi;
            }
            false
        }
        "min-color" => {
            // FIXME: assuming 8 bits per color component
            if let css::Value::Number(bits) = &declaration.value {
                return &8 >= bits;
            }
            false
        }
        "max-color" => {
            // FIXME: assuming 8 bits per color component
            if let css::Value::Number(bits) = &declaration.value {
                return &8 <= bits;
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod parse {
    use super::*;

    #[test]
    fn check_ratio11() {
        let declaration = css::Declaration {
            important: false,
            name: String::from("aspect-ratio"),
            value: Value::Ratio(1, 1),
        };
        assert!(declaration_matches(&declaration, (100, 100)));
        assert!(!declaration_matches(&declaration, (110, 100)));
    }

    #[test]
    fn check_ratio32() {
        let declaration = css::Declaration {
            important: false,
            name: String::from("aspect-ratio"),
            value: Value::Ratio(3, 2),
        };
        assert!(declaration_matches(&declaration, (300, 200)));
        assert!(!declaration_matches(&declaration, (200, 300)));
    }

    #[test]
    fn check_min_ratio32() {
        let declaration = css::Declaration {
            important: false,
            name: String::from("min-aspect-ratio"),
            value: Value::Ratio(3, 2),
        };
        assert!(declaration_matches(&declaration, (400, 100)));
        assert!(declaration_matches(&declaration, (600, 200)));
        assert!(!declaration_matches(&declaration, (100, 100)));
    }

    #[test]
    fn check_max_ratio32() {
        let declaration = css::Declaration {
            important: false,
            name: String::from("max-aspect-ratio"),
            value: Value::Ratio(3, 2),
        };
        assert!(declaration_matches(&declaration, (100, 100)));
        assert!(declaration_matches(&declaration, (110, 100)));
        assert!(!declaration_matches(&declaration, (300, 200)));
        assert!(!declaration_matches(&declaration, (800, 200)));
    }

    #[test]
    fn check_grid() {
        // bitmap based
        let declaration1 = css::Declaration {
            important: false,
            name: String::from("grid"),
            value: Value::Number(0),
        };
        assert!(declaration_matches(&declaration1, (0, 0)));

        // grid based
        let declaration2 = css::Declaration {
            important: false,
            name: String::from("grid"),
            value: Value::Number(1),
        };
        assert!(!declaration_matches(&declaration2, (0, 0)));
    }

    #[test]
    fn check_resolution() {
        let declaration1 = css::Declaration {
            important: false,
            name: String::from("resolution"),
            value: Value::Number(96),
        };
        assert!(declaration_matches(&declaration1, (0, 0)));

        let declaration2 = css::Declaration {
            important: false,
            name: String::from("resolution"),
            value: Value::Number(95),
        };
        assert!(!declaration_matches(&declaration2, (0, 0)));
    }

    #[test]
    fn check_min_max_resolution() {
        let declaration_min1 = css::Declaration {
            important: false,
            name: String::from("min-resolution"),
            value: Value::Number(96),
        };
        assert!(declaration_matches(&declaration_min1, (0, 0)));

        let declaration_min2 = css::Declaration {
            important: false,
            name: String::from("min-resolution"),
            value: Value::Number(97),
        };
        assert!(!declaration_matches(&declaration_min2, (0, 0)));

        let declaration_max1 = css::Declaration {
            important: false,
            name: String::from("max-resolution"),
            value: Value::Number(96),
        };
        assert!(declaration_matches(&declaration_max1, (0, 0)));

        let declaration_max2 = css::Declaration {
            important: false,
            name: String::from("max-resolution"),
            value: Value::Number(95),
        };
        assert!(!declaration_matches(&declaration_max2, (0, 0)));
    }

    #[test]
    fn check_color() {
        let feature = MediaFeature::Name(String::from("color"), false);
        assert!(feature_matches(&feature, (0, 0)));
    }

    #[test]
    fn check_color_not() {
        let feature = MediaFeature::Name(String::from("color"), true);
        assert!(!feature_matches(&feature, (0, 0)));
    }

    #[test]
    fn check_min_max_color() {
        let declaration_min1 = css::Declaration {
            important: false,
            name: String::from("min-color"),
            value: Value::Number(1),
        };
        assert!(declaration_matches(&declaration_min1, (0, 0)));

        let declaration_min2 = css::Declaration {
            important: false,
            name: String::from("min-color"),
            value: Value::Number(9),
        };
        assert!(!declaration_matches(&declaration_min2, (0, 0)));

        let declaration_max1 = css::Declaration {
            important: false,
            name: String::from("max-color"),
            value: Value::Number(1),
        };
        assert!(!declaration_matches(&declaration_max1, (0, 0)));

        let declaration_max2 = css::Declaration {
            important: false,
            name: String::from("max-color"),
            value: Value::Number(9),
        };
        assert!(declaration_matches(&declaration_max2, (0, 0)));
    }
}
