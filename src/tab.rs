use crate::css;
use crate::data_storage;
use crate::display;
use crate::dom;
use crate::html;
use crate::layout::lbox::LBox;
use crate::logic;
use crate::style;
use crate::stylednode;

use std::collections::HashMap;

pub struct Tab {
    pub url: String,
    pub display_list: Vec<display::DisplayCommand>,
    pub style_root: Option<stylednode::StyledNode>,
    pub layout_height: f32,
    pub scrolled: f32,
    pub history: Vec<String>,
    his_cursor: usize,
    pub title: Option<String>,
    pub layout: Option<LBox>,
    /// theme color of page
    pub color: Option<css::Color>,
    /// path of favicon icon
    pub favicon: Option<String>,
}

impl Tab {
    pub fn new() -> Self {
        Self {
            url: String::new(),
            display_list: Vec::new(),
            style_root: None,
            layout_height: 0.0,
            scrolled: 0.0,
            history: Vec::new(),
            his_cursor: 0,
            title: Some(String::from("New Tab")),
            layout: None,
            color: None,
            favicon: Some(String::from("assets/icon.png")),
        }
    }

    /// go 1 forward in tab history
    pub fn go_forward(&mut self, dimensions: (u32, u32)) {
        let hist_len = self.history.len();
        if hist_len > 1 && hist_len - self.his_cursor > 0 {
            self.open(self.history[self.his_cursor].clone(), dimensions);
            self.his_cursor += 1;
        }
    }

    /// go 1 back in tab history
    pub fn go_back(&mut self, dimensions: (u32, u32)) {
        let hist_len = self.history.len();
        if hist_len > 1 && hist_len - self.his_cursor + 1 > 0 && self.his_cursor != 1 {
            self.his_cursor -= 1;
            self.open(self.history[self.his_cursor - 1].clone(), dimensions);
        }
    }

    /// open an url
    /// no history
    pub fn open(&mut self, mut url_to_open: String, dimensions: (u32, u32)) {
        let mut html_source;
        // FIXME: don't reload resource if only `#bookmark_id` changes

        if url_to_open.is_empty() {
            self.title = Some(String::from("New Tab"));
            return;
        } else if url_to_open.starts_with("view-source:") {
            let mut title = String::from("Source: ");
            title.push_str(&url_to_open[12..]);
            self.title = Some(title);
            self.url = url_to_open.clone();

            let mut source = data_storage::download_and_get(&url_to_open[12..], vec!["text/html"])
                .expect("download to work");
            source = source.replace("\t", "    ");

            let lines: Vec<&str> = source.split('\n').collect();

            let mut children = Vec::new();

            for line in lines {
                if line.is_empty() {
                    continue;
                }

                let mut div_style = HashMap::new();
                div_style.insert(String::from("style"), String::from("display:block"));
                let div = dom::Node::elem(
                    String::from("div"),
                    div_style,
                    vec![dom::Node::text(line.to_string())],
                );
                children.push(div);
            }

            let mut style = HashMap::new();
            style.insert(String::from("style"), String::from("font-family:monospace"));

            let root_node = dom::Node::elem(String::from("html"), style, children);

            let style_root = style::style_tree(
                root_node,
                &Vec::new(),
                &HashMap::new(),
                vec![Vec::new()],
                &url_to_open,
            );

            let layout =
                display::layout(style_root.clone(), dimensions.0 as f32, dimensions.1 as f32);
            self.layout_height = layout.dimensions.margin_box().height;
            self.display_list = display::build_display_list(&layout);

            self.layout = Some(layout);
            self.style_root = Some(style_root);
            return;
        } else if url_to_open.starts_with("gopher://") {
            dbg!("maybe TODO: gopher");
            return;
        } else if url_to_open.starts_with("gemini://") {
            dbg!("maybe TODO: gemini");
            return;
        } else if url_to_open.starts_with("finger://") {
            dbg!("maybe TODO: finger");
            return;
        } else if url_to_open.starts_with("file://") {
            dbg!("TODO: open local file");
            return;
        } else if !url_to_open.contains(' ')
            && url_to_open.contains('.')
            && !url_to_open.starts_with('.')
            && !url_to_open.ends_with('.')
        {
            if !(url_to_open.starts_with("https://") || url_to_open.starts_with("http://")) {
                url_to_open = format!("https://{}", url_to_open);
            }
            if url_to_open.starts_with("https://dev.dev") {
                println!("loading dev resource");
                html_source = data_storage::open_local_file("assets/dev.html")
                    .expect("dev' asset to be present");
            } else {
                html_source = data_storage::for_tab(&url_to_open);
            }
        } else {
            let params = [("q", &*url_to_open), ("kl", "us-en")];
            html_source =
                data_storage::download_and_get_post("https://duckduckgo.com/lite/", &params);
            url_to_open = String::from("https://duckduckgo.com/lite/");
        }

        self.url = url_to_open.clone();

        /* response_body = response_body
        .replace("\x0D\x0A", "\n")
        .replace("\x0D", "\n")
        .replace("\x0C", "\n")
        .replace("\x00", "\n");*/
        html_source = html_source.replace("\t", " ").replace("\n", "");

        let (root_node, raw_stylesheets) = html::parse(html_source, url_to_open.clone());
        let default_css = data_storage::open_local_file("assets/default-style.css")
            .expect("'default-style' asset to be present");
        let mut stylesheets = vec![css::parse(default_css, String::new(), dimensions)];

        for sheet in raw_stylesheets {
            match sheet {
                (style, None) => {
                    stylesheets.push(css::parse(style, url_to_open.clone(), dimensions));
                }
                (sheet_url, Some(query)) => {
                    let mut parser = css::media_query::parser::Parser {
                        pos: 0,
                        input: query,
                    };

                    if parser.matches(dimensions) {
                        if let Ok(style) =
                            data_storage::download_and_get(&sheet_url, vec!["text/css"])
                        {
                            stylesheets.push(css::parse(style, sheet_url, dimensions));
                        }
                    }
                }
            }
        }

        let style_root = style::style_tree(
            root_node,
            &stylesheets,
            &HashMap::new(),
            vec![Vec::new()],
            &url_to_open,
        );

        let possible_title_node = style_root.finde_node("title", None);
        if let Some(title_node) = possible_title_node {
            // FIXME: assuming `<title>` has a text node as only child. Might break if no child or wont work if title is wrapped in other element
            if let dom::NodeType::Text(title) = &title_node.children[0].node_type {
                self.title = Some(title[0].clone());
            } else {
                self.title = None;
            }
        } else {
            self.title = None;
        }

        {
            // FIXME: move this somewhere else. Don't block rendering
            // tab color `<meta name="theme-color" content="#333">`
            let possible_color = style_root.finde_node("meta", Some(("name", "theme-color")));
            if let Some(meta_node) = possible_color {
                if let dom::NodeType::Element(element_data) = &meta_node.node_type {
                    if let Some(value) = element_data.attributes.get("content") {
                        let mut parser = css::parser::Parser {
                            pos: 0,
                            input: value.clone(),
                            url: String::new(),
                        };

                        if let Some(css::Value::Color(color)) = parser.parse_value() {
                            self.color = Some(color);
                        }
                    }
                }
            }
        }

        // favicon
        let favicon_url = logic::absolute_path(&self.url, "/favicon.ico");
        // FIXME: add possible other favicon positons https://en.wikipedia.org/wiki/Favicon#How_to_use
        self.favicon = data_storage::download_cache_path(&favicon_url, vec!["image/x-icon"]).ok();

        {
            // FIXME: move this somewhere else. Don't block rendering
            // FIXME: there can be multiple feed for different things
            // FIXME: display icon in GUI where link can be shown
            // rss feed detection
            let atom = style_root.finde_node("link", Some(("type", "application/atom+xml")));
            if let Some(atom_node) = atom {
                if let dom::NodeType::Element(element_data) = &atom_node.node_type {
                    if let Some(value) = element_data.attributes.get("href") {
                        println!("Atom feed found at {}", value);
                    }
                }
            }

            let rss = style_root.finde_node("link", Some(("type", "application/rss+xml")));
            if let Some(rss_node) = rss {
                if let dom::NodeType::Element(element_data) = &rss_node.node_type {
                    if let Some(value) = element_data.attributes.get("href") {
                        println!("RSS feed found at {}", value);
                    }
                }
            }

            let json = style_root.finde_node("link", Some(("type", "application/feed+json")));
            if let Some(json_node) = json {
                if let dom::NodeType::Element(element_data) = &json_node.node_type {
                    if let Some(value) = element_data.attributes.get("href") {
                        println!("JSON feed found at {}", value);
                    }
                }
            }
        }

        let layout = display::layout(style_root.clone(), dimensions.0 as f32, dimensions.1 as f32);
        self.layout_height = layout.dimensions.margin_box().height;
        self.display_list = display::build_display_list(&layout);

        // scroll to bookmark link
        {
            if let Some(pos) = self.url.find('#') {
                let id = &self.url[pos + 1..];

                if !id.is_empty() {
                    let y = if id == "top" {
                        Some(0.0)
                    } else {
                        let possible_bookmark = layout.finde_box_id(id);
                        if let Some(bookmark) = possible_bookmark {
                            Some(bookmark.dimensions.content.y)
                        } else {
                            None
                        }
                    };

                    if let Some(scroll) = y {
                        self.scrolled = scroll;
                        display::scroll(&mut self.display_list, -scroll);
                    }
                }
            }
        }

        self.layout = Some(layout);
        self.style_root = Some(style_root);
    }

    /// browse to an url, appending url to tab history
    pub fn browse(&mut self, url_to_open: String, dimensions: (u32, u32)) {
        self.open(url_to_open.clone(), dimensions);

        self.history.push(url_to_open);
        self.his_cursor += 1;
    }
}
