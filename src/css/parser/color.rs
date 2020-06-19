use crate::css::parser::Parser;
use crate::css::{hue2rgb, valid_hex_char, Clamp, Color, Value};

impl Parser {
    /// parses `(255, 123, 0` or `(100%, 10%, 0%`
    pub fn parse_rgb(&mut self) -> (u8, u8, u8) {
        let mut percentage = false;
        self.consume_char(); // (
        self.consume_blank();

        let red = self.consume_while(|c| match c {
            '0'..='9' | '-' | '.' => true,
            _ => false,
        });

        if let Some('%') = self.next_char() {
            self.consume_char(); // %
            percentage = true;
        }

        self.consume_blank();
        self.consume_char(); // ,
        self.consume_blank();

        let green = self.consume_while(|c| match c {
            '0'..='9' | '-' | '.' => true,
            _ => false,
        });
        if percentage {
            self.consume_char(); // %
        }
        self.consume_blank();
        self.consume_char(); // ,
        self.consume_blank();

        let blue = self.consume_while(|c| match c {
            '0'..='9' | '-' | '.' => true,
            _ => false,
        });
        if percentage {
            self.consume_char(); // %
        }
        self.consume_blank();

        let mut red: f32 = red.parse().unwrap();
        let mut green: f32 = green.parse().unwrap();
        let mut blue: f32 = blue.parse().unwrap();

        if percentage {
            red = red / 100.0 * 255.0;
            green = green / 100.0 * 255.0;
            blue = blue / 100.0 * 255.0;
        }

        (
            red.clamp_value(0.0, 255.0),
            green.clamp_value(0.0, 255.0),
            blue.clamp_value(0.0, 255.0),
        )
    }

    /// parses hsl values. leaves end open for hsla
    /// e.g. `(233, 40.5%, 23%`
    /// <https://www.w3.org/TR/css-color-3/#hsl-color>
    /// <https://drafts.csswg.org/css-color/#hsl-to-rgb>
    pub fn parse_hsl(&mut self) -> (u8, u8, u8) {
        self.consume_char(); // (
        self.consume_blank();

        let hue = self.consume_while(|c| match c {
            '0'..='9' | '.' => true,
            _ => false,
        });

        self.consume_blank();
        self.consume_char(); // ,
        self.consume_blank();

        let saturation = self.consume_while(|c| match c {
            '0'..='9' | '.' => true,
            _ => false,
        });
        self.consume_char(); // %
        self.consume_blank();
        self.consume_char(); // ,
        self.consume_blank();

        let lightness = self.consume_while(|c| match c {
            '0'..='9' | '.' => true,
            _ => false,
        });
        self.consume_char(); // %
        self.consume_blank();

        let hue = hue.parse::<f32>().unwrap() / 360.0;
        let saturation = saturation.parse::<f32>().unwrap() / 100.0;
        let lightness = lightness.parse::<f32>().unwrap() / 100.0;

        // http://www.easyrgb.com/en/math.php#text19
        // https://stackoverflow.com/questions/2353211/hsl-to-rgb-color-conversion
        let q = if lightness < 0.5 {
            lightness * (1.0 + saturation)
        } else {
            lightness + saturation - lightness * saturation
        };
        let p = 2.0 * lightness - q;

        let red = if saturation == 0.0 {
            lightness
        } else {
            hue2rgb(p, q, hue + 1.0 / 3.0)
        };
        let green = if saturation == 0.0 {
            lightness
        } else {
            hue2rgb(p, q, hue)
        };
        let blue = if saturation == 0.0 {
            lightness
        } else {
            hue2rgb(p, q, hue - 1.0 / 3.0)
        };

        (
            (red * 255.0).round() as u8,
            (green * 255.0).round() as u8,
            (blue * 255.0).round() as u8,
        )
    }

    /// Parses a hex color like `#f02`.
    pub fn parse_hex_color(&mut self) -> Option<Value> {
        self.consume_char(); // #

        let hex = self.parse_valid_hex();

        // https://drafts.csswg.org/css-color/#hex-notation
        match hex.len() {
            3 => Some(Value::Color(Color {
                r: u8::from_str_radix(&*format!("{}{}", &hex[0..1], &hex[0..1]), 16).unwrap(),
                g: u8::from_str_radix(&*format!("{}{}", &hex[1..2], &hex[1..2]), 16).unwrap(),
                b: u8::from_str_radix(&*format!("{}{}", &hex[2..3], &hex[2..3]), 16).unwrap(),
                a: 255,
            })),
            4 => Some(Value::Color(Color {
                r: u8::from_str_radix(&*format!("{}{}", &hex[0..1], &hex[0..1]), 16).unwrap(),
                g: u8::from_str_radix(&*format!("{}{}", &hex[1..2], &hex[1..2]), 16).unwrap(),
                b: u8::from_str_radix(&*format!("{}{}", &hex[2..3], &hex[2..3]), 16).unwrap(),
                a: u8::from_str_radix(&*format!("{}{}", &hex[3..4], &hex[3..4]), 16).unwrap(),
            })),
            6 => Some(Value::Color(Color {
                r: u8::from_str_radix(&hex[..2], 16).unwrap(),
                g: u8::from_str_radix(&hex[2..4], 16).unwrap(),
                b: u8::from_str_radix(&hex[4..6], 16).unwrap(),
                a: 255,
            })),
            8 => Some(Value::Color(Color {
                r: u8::from_str_radix(&hex[..2], 16).unwrap(),
                g: u8::from_str_radix(&hex[2..4], 16).unwrap(),
                b: u8::from_str_radix(&hex[4..6], 16).unwrap(),
                a: u8::from_str_radix(&hex[6..8], 16).unwrap(),
            })),
            _ => None,
        }
    }

    /// Parse a hex color.
    /// `'a'..='f' | 'A'..='F' | '0'..='9'`
    fn parse_valid_hex(&mut self) -> String {
        self.consume_while(valid_hex_char)
    }
}

#[cfg(test)]
mod parse_element {
    use super::*;

    #[test]
    fn hex8() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("#17977623"),
            url: String::new(),
        };

        assert_eq!(
            parser.parse_hex_color(),
            Some(Value::Color(Color {
                r: 23,
                g: 151,
                b: 118,
                a: 35
            }))
        );
    }

    #[test]
    fn hex6() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("#afb033"),
            url: String::new(),
        };

        assert_eq!(
            parser.parse_hex_color(),
            Some(Value::Color(Color {
                r: 175,
                g: 176,
                b: 51,
                a: 255
            }))
        );
    }

    #[test]
    fn hex4() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("#8c84"),
            url: String::new(),
        };

        assert_eq!(
            parser.parse_hex_color(),
            Some(Value::Color(Color {
                r: 136,
                g: 204,
                b: 136,
                a: 68
            }))
        );
    }

    #[test]
    fn hex3() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("#c77"),
            url: String::new(),
        };

        assert_eq!(
            parser.parse_hex_color(),
            Some(Value::Color(Color {
                r: 204,
                g: 119,
                b: 119,
                a: 255
            }))
        );
    }

    #[test]
    fn hex_alpha() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("#0f3"),
            url: String::new(),
        };

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("#00ff33ff"),
            url: String::new(),
        };

        assert_eq!(parser1.parse_hex_color(), parser2.parse_hex_color(),);
    }

    #[test]
    fn hex84() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("#2f08"),
            url: String::new(),
        };

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("#22ff0088"),
            url: String::new(),
        };

        assert_eq!(parser1.parse_hex_color(), parser2.parse_hex_color(),);
    }

    #[test]
    fn hex63() {
        let mut parser1 = Parser {
            pos: 0,
            input: String::from("#904"),
            url: String::new(),
        };

        let mut parser2 = Parser {
            pos: 0,
            input: String::from("#990044"),
            url: String::new(),
        };

        assert_eq!(parser1.parse_hex_color(), parser2.parse_hex_color(),);
    }

    #[test]
    fn hex_none() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("hallo"),
            url: String::new(),
        };

        assert_eq!(parser.parse_hex_color(), None);
    }

    #[test]
    fn hex_hsl() {
        let mut parser = Parser {
            pos: 0,
            input: String::from("(0,0%,93.3%"),
            url: String::new(),
        };

        assert_eq!(parser.parse_hsl(), (238, 238, 238));
    }

    /*
    TODO: handle and test all those cases:
        // Old Syntax
        rgb(0, 128, 255)

        rgba(0, 128, 255, 0.5)

        hsl(198, 38% 50%)

        hsla(198, 28%, 50%, 0.5)

        // New Syntax
        rgb(0 128 255)

        rgb(0 128 255 / 50%)

        hsl(198deg 28% 50%)

        hsl(198deg 28% 50% / 50%)

        lab(56.29% -10.93 16.58 / 50%)

        lch(56.29% 19.86 236.62 / 50%)

        color(sRGB 0 0.50 1 / 50%)


        /* These examples all specify the same color: a lavender. */
        hsl(270,60%,70%)
        hsl(270, 60%, 70%)
        hsl(270 60% 70%)
        hsl(270deg, 60%, 70%)
        hsl(4.71239rad, 60%, 70%)
        hsl(.75turn, 60%, 70%)

        /* These examples all specify the same color: a lavender that is 15% opaque. */
        hsl(270, 60%, 50%, .15)
        hsl(270, 60%, 50%, 15%)
        hsl(270 60% 50% / .15)
        hsl(270 60% 50% / 15%)
    */
}
