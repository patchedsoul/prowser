pub mod media_query;
pub mod parser;

use crate::layout;

#[derive(Debug)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct Rule {
    pub declarations: Vec<Declaration>,
    pub selectors: Vec<ChainedSelector>,
}

/// Css `<selector>` like `#id.class`
///
/// A `SimpleSelector` is either a type selector, universal selector, attribute selector, class selector, ID selector, or pseudo-class.
#[derive(Debug, Clone)]
pub struct SimpleSelector {
    pub attribute: Vec<(String, char, String)>,
    pub class: Vec<String>,
    pub id: Option<String>,
    pub tag_name: Option<String>,
}

/// Vec(`selector` [+ `kombinator`])
#[derive(Debug)]
pub struct ChainedSelector {
    pub selectors: Vec<(SimpleSelector, char)>,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub important: bool,
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Color(Color),
    Gradient(u16, Vec<Color>),
    Keyword(String),
    Length(f32, Unit),
    Str(String),
    Url(String),
    Ratio(u32, u32),
    Number(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Ch, // width of the "0" (ZERO, U+0030) glyph in the element’s font
    Cm,
    Em,
    Ex, // x-height of the element’s font
    In,
    Mm,
    Pc,
    Percentage,
    Pt,
    Px,
    Q,
    Rem,
    Vh,
    Vmax,
    Vmin,
    Vw,
    Zero,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

///                important, inline, id, class/attribute, tag name
pub type Specificity = (bool, bool, usize, usize, usize);

impl ChainedSelector {
    /// calculates specificity
    ///
    /// <http://www.w3.org/TR/selectors/#specificity>
    pub fn specificity(&self) -> Specificity {
        let mut a = 0;
        let mut b = 0;
        let mut c = 0;

        for (simple, _) in &self.selectors {
            a += simple.id.iter().count();
            b += simple.class.len() + simple.attribute.len();
            c += simple.tag_name.iter().count();
        }

        (false, false, a, b, c)
    }
}

impl Value {
    /// Return the size of a length in px, or zero for non-lengths.
    /// <https://drafts.csswg.org/css-values-3/#absolute-lengths>
    ///
    ///                 ↓ relavtive Value for Percentage calculation
    pub fn to_px(&self, per: f32, root_block: &layout::Dimensions) -> f32 {
        match *self {
			Self::Length(f, Unit::Ch) => f * 10.0, // FIXME: calculate correctly
			Self::Length(f, Unit::Cm) => f * 96.0 / 2.54, // centimeters (1cm = 1/2.54in)
			Self::Length(f, Unit::Pc) // picas (1pc = 12 pt)
			| Self::Length(f, Unit::Em)
			| Self::Length(f, Unit::Rem) => f * 16.0, // FIXME: depending on font
			Self::Length(f, Unit::Ex) => f * 8.0,  // FIXME: calculate correctly
			Self::Length(f, Unit::In) => f * 96.0, // inches (1in = 96px)
			Self::Length(f, Unit::Mm) => f * 96.0 / 25.4, // millimeters (1mm = 1/25.4in)
			Self::Length(f, Unit::Percentage) => per / 100.0 * f,
			Self::Length(f, Unit::Pt) => f * 96.0 / 72.0, // points (1pt = 1/72in)
			Self::Length(f, Unit::Px) => f,
			Self::Length(f, Unit::Q) => f * 2.4 / 2.54, // quarter-millimeters (1Q = 1/40cm)
			Self::Length(f, Unit::Vh) => root_block.content.height / 100.0 * f,
			Self::Length(f, Unit::Vmax) => {
				let vh = root_block.content.height / 100.0 * f;
				let vw = root_block.content.width / 100.0 * f;
				if vh > vw {
					vh
				} else {
					vw
				}
			}
			Self::Length(f, Unit::Vmin) => {
				let vh = root_block.content.height / 100.0 * f;
				let vw = root_block.content.width / 100.0 * f;
				if vh < vw {
					vh
				} else {
					vw
				}
			}
			Self::Length(f, Unit::Vw) => root_block.content.width / 100.0 * f,
			_ => 0.0,
		}
    }
}

trait Clamp {
    fn clamp_value(self, lower: Self, upper: Self) -> u8;
}
impl Clamp for f32 {
    fn clamp_value(self, lower: Self, upper: Self) -> u8 {
        self.max(lower).min(upper).round() as u8
    }
}

/// Parse a whole CSS stylesheet.
pub fn parse(source: String, url: String, dimensions: (u32, u32)) -> Stylesheet {
    let mut parser = parser::Parser {
        pos: 0,
        input: source,
        url,
    };
    Stylesheet {
        rules: parser.parse_rules(dimensions),
    }
}

/// `'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_'`
fn valid_identifier_char(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => true, // TODO: Include U+00A0 and higher. Warum?
        _ => false,
    }
}

/// `'a'..='z' | 'A'..='Z' | '%'`
fn valid_unit_char(c: char) -> bool {
    match c {
        'a'..='z' | 'A'..='Z' | '%' => true,
        _ => false,
    }
}

/// `'a'..='f' | 'A'..='F' | '0'..='9'`
fn valid_hex_char(c: char) -> bool {
    match c {
        'a'..='f' | 'A'..='F' | '0'..='9' => true,
        _ => false,
    }
}

/// converts hue to rgb
/// [color-math](http://www.easyrgb.com/en/math.php#text19)
/// [stack](https://stackoverflow.com/questions/2353211/hsl-to-rgb-color-conversion)
fn hue2rgb(p: f32, q: f32, mut h: f32) -> f32 {
    if h < 0.0 {
        h += 1.0;
    }
    if h > 1.0 {
        h -= 1.0;
    }

    if h < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * h;
    }
    if h < 1.0 / 2.0 {
        return q;
    }
    if h < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - h) * 6.0;
    }
    p
}

/// checks weather a keyword is a color
fn check_color_keyword(keyword: &str) -> Option<Value> {
    if keyword == "transparent" {
        return Some(Value::Color(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }));
    }

    // 17 base colors + 130 extended colors
    // https://www.w3.org/TR/css-color-3/
    let color_keywords = [
        ("aliceblue", (240, 248, 255)),
        ("antiquewhite", (250, 235, 215)),
        ("aqua", (0, 255, 255)),
        ("aquamarine", (127, 255, 212)),
        ("azure", (240, 255, 255)),
        ("beige", (245, 245, 220)),
        ("bisque", (255, 228, 196)),
        ("black", (0, 0, 0)),
        ("blanchedalmond", (255, 235, 205)),
        ("blue", (0, 0, 255)),
        ("blueviolet", (138, 43, 226)),
        ("brown", (165, 42, 42)),
        ("burlywood", (222, 184, 135)),
        ("cadetblue", (95, 158, 160)),
        ("chartreuse", (127, 255, 0)),
        ("chocolate", (210, 105, 30)),
        ("coral", (255, 127, 80)),
        ("cornflowerblue", (100, 149, 237)),
        ("cornsilk", (255, 248, 220)),
        ("crimson", (220, 20, 60)),
        ("cyan", (0, 255, 255)),
        ("darkblue", (0, 0, 139)),
        ("darkcyan", (0, 139, 139)),
        ("darkgoldenrod", (184, 134, 11)),
        ("darkgray", (169, 169, 169)),
        ("darkgreen", (0, 100, 0)),
        ("darkgrey", (169, 169, 169)),
        ("darkkhaki", (189, 183, 107)),
        ("darkmagenta", (139, 0, 139)),
        ("darkolivegreen", (85, 107, 47)),
        ("darkorange", (255, 140, 0)),
        ("darkorchid", (153, 50, 204)),
        ("darkred", (139, 0, 0)),
        ("darksalmon", (233, 150, 122)),
        ("darkseagreen", (143, 188, 143)),
        ("darkslateblue", (72, 61, 139)),
        ("darkslategray", (47, 79, 79)),
        ("darkslategrey", (47, 79, 79)),
        ("darkturquoise", (0, 206, 209)),
        ("darkviolet", (148, 0, 211)),
        ("deeppink", (255, 20, 147)),
        ("deepskyblue", (0, 191, 255)),
        ("dimgray", (105, 105, 105)),
        ("dimgrey", (105, 105, 105)),
        ("dodgerblue", (30, 144, 255)),
        ("firebrick", (178, 34, 34)),
        ("floralwhite", (255, 250, 240)),
        ("forestgreen", (34, 139, 34)),
        ("fuchsia", (255, 0, 255)),
        ("gainsboro", (220, 220, 220)),
        ("ghostwhite", (248, 248, 255)),
        ("gold", (255, 215, 0)),
        ("goldenrod", (218, 165, 32)),
        ("gray", (128, 128, 128)),
        ("green", (0, 128, 0)),
        ("greenyellow", (173, 255, 47)),
        ("grey", (128, 128, 128)),
        ("honeydew", (240, 255, 240)),
        ("hotpink", (255, 105, 180)),
        ("indianred", (205, 92, 92)),
        ("indigo", (75, 0, 130)),
        ("ivory", (255, 255, 240)),
        ("khaki", (240, 230, 140)),
        ("lavender", (230, 230, 250)),
        ("lavenderblush", (255, 240, 245)),
        ("lawngreen", (124, 252, 0)),
        ("lemonchiffon", (255, 250, 205)),
        ("lightblue", (173, 216, 230)),
        ("lightcoral", (240, 128, 128)),
        ("lightcyan", (224, 255, 255)),
        ("lightgoldenrodyellow", (250, 250, 210)),
        ("lightgray", (211, 211, 211)),
        ("lightgreen", (144, 238, 144)),
        ("lightgrey", (211, 211, 211)),
        ("lightpink", (255, 182, 193)),
        ("lightsalmon", (255, 160, 122)),
        ("lightseagreen", (32, 178, 170)),
        ("lightskyblue", (135, 206, 250)),
        ("lightslategray", (119, 136, 153)),
        ("lightslategrey", (119, 136, 153)),
        ("lightsteelblue", (176, 196, 222)),
        ("lightyellow", (255, 255, 224)),
        ("lime", (0, 255, 0)),
        ("limegreen", (50, 205, 50)),
        ("linen", (250, 240, 230)),
        ("magenta", (255, 0, 255)),
        ("maroon", (128, 0, 0)),
        ("mediumaquamarine", (102, 205, 170)),
        ("mediumblue", (0, 0, 205)),
        ("mediumorchid", (186, 85, 211)),
        ("mediumpurple", (147, 112, 219)),
        ("mediumseagreen", (60, 179, 113)),
        ("mediumslateblue", (123, 104, 238)),
        ("mediumspringgreen", (0, 250, 154)),
        ("mediumturquoise", (72, 209, 204)),
        ("mediumvioletred", (199, 21, 133)),
        ("midnightblue", (25, 25, 112)),
        ("mintcream", (245, 255, 250)),
        ("mistyrose", (255, 228, 225)),
        ("moccasin", (255, 228, 181)),
        ("navajowhite", (255, 222, 173)),
        ("navy", (0, 0, 128)),
        ("oldlace", (253, 245, 230)),
        ("olive", (128, 128, 0)),
        ("olivedrab", (107, 142, 35)),
        ("orange", (255, 165, 0)),
        ("orangered", (255, 69, 0)),
        ("orchid", (218, 112, 214)),
        ("palegoldenrod", (238, 232, 170)),
        ("palegreen", (152, 251, 152)),
        ("paleturquoise", (175, 238, 238)),
        ("palevioletred", (219, 112, 147)),
        ("papayawhip", (255, 239, 213)),
        ("peachpuff", (255, 218, 185)),
        ("peru", (205, 133, 63)),
        ("pink", (255, 192, 203)),
        ("plum", (221, 160, 221)),
        ("powderblue", (176, 224, 230)),
        ("purple", (128, 0, 128)),
        ("red", (255, 0, 0)),
        ("rosybrown", (188, 143, 143)),
        ("royalblue", (65, 105, 225)),
        ("saddlebrown", (139, 69, 19)),
        ("salmon", (250, 128, 114)),
        ("sandybrown", (244, 164, 96)),
        ("seagreen", (46, 139, 87)),
        ("seashell", (255, 245, 238)),
        ("sienna", (160, 82, 45)),
        ("silver", (192, 192, 192)),
        ("skyblue", (135, 206, 235)),
        ("slateblue", (106, 90, 205)),
        ("slategray", (112, 128, 144)),
        ("slategrey", (112, 128, 144)),
        ("snow", (255, 250, 250)),
        ("springgreen", (0, 255, 127)),
        ("steelblue", (70, 130, 180)),
        ("tan", (210, 180, 140)),
        ("teal", (0, 128, 128)),
        ("thistle", (216, 191, 216)),
        ("tomato", (255, 99, 71)),
        ("turquoise", (64, 224, 208)),
        ("violet", (238, 130, 238)),
        ("wheat", (245, 222, 179)),
        ("white", (255, 255, 255)),
        ("whitesmoke", (245, 245, 245)),
        ("yellow", (255, 255, 0)),
        ("yellowgreen", (154, 205, 50)),
    ];

    // FIXME: maybe a match (`"red" => (255,0,0)`). could be faster and less ram hungry?
    // https://siciarz.net/24-days-rust-static-initialization/

    let mut iter = color_keywords.iter();
    let color_keyword = iter.find(|(x, _)| x == &keyword);

    color_keyword.map(|(_, values)| {
        Value::Color(Color {
            r: values.0,
            g: values.1,
            b: values.2,
            a: 255,
        })
    })
}

#[cfg(test)]
mod specifity {
    use super::*;

    #[test]
    fn zero() {
        let simple = SimpleSelector {
            attribute: Vec::new(),
            class: Vec::new(),
            id: None,
            tag_name: None,
        };
        let chained = ChainedSelector {
            selectors: vec![(simple, 0 as char)],
        };

        assert_eq!(chained.specificity(), (false, false, 0, 0, 0));
    }

    #[test]
    fn id() {
        let simple = SimpleSelector {
            attribute: Vec::new(),
            class: Vec::new(),
            id: Some(String::from("a")),
            tag_name: None,
        };
        let chained = ChainedSelector {
            selectors: vec![(simple, 0 as char)],
        };

        assert_eq!(chained.specificity(), (false, false, 1, 0, 0));
    }

    #[test]
    fn class() {
        let simple = SimpleSelector {
            attribute: Vec::new(),
            class: vec![String::from("a"), String::from("b")],
            id: None,
            tag_name: None,
        };
        let chained = ChainedSelector {
            selectors: vec![(simple, 0 as char)],
        };

        assert_eq!(chained.specificity(), (false, false, 0, 2, 0));
    }

    #[test]
    fn mixed() {
        let simple = SimpleSelector {
            attribute: vec![(String::from("s"), 'a', String::from("d"))],
            class: vec![String::from("a"), String::from("b")],
            id: Some(String::from("c")),
            tag_name: Some(String::from("div")),
        };
        let chained = ChainedSelector {
            selectors: vec![(simple, 0 as char)],
        };

        assert_eq!(chained.specificity(), (false, false, 1, 3, 1));
    }

    #[test]
    fn chained() {
        let simple1 = SimpleSelector {
            attribute: Vec::new(),                             // 0, 0, 0
            class: vec![String::from("a"), String::from("b")], // 0, 2, 0
            id: Some(String::from("c")),                       // 1, 0, 0
            tag_name: Some(String::from("div")),               // 0, 0, 1
        };
        let simple2 = SimpleSelector {
            attribute: vec![(String::from("s"), 'a', String::from("d"))], // 0, 1, 0
            class: vec![String::from("a")],                               // 0, 1, 0
            id: Some(String::from("c")),                                  // 1, 0, 0
            tag_name: None,                                               // 0, 0, 0
        };
        let chained = ChainedSelector {
            selectors: vec![(simple1, '>'), (simple2, '0')],
        };

        assert_eq!(chained.specificity(), (false, false, 2, 4, 1));
    }

    #[test]
    fn color_keyword_none() {
        assert_eq!(check_color_keyword(""), None);
    }

    #[test]
    fn color_keyword_transparent() {
        assert_eq!(
            check_color_keyword("transparent"),
            Some(Value::Color(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0
            }))
        );
    }

    #[test]
    fn color_keyword_lavenderblush() {
        assert_eq!(
            check_color_keyword("lavenderblush"),
            Some(Value::Color(Color {
                r: 255,
                g: 240,
                b: 245,
                a: 255
            }))
        );
    }
}
