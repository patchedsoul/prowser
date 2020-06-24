mod css;
mod data_storage;
mod display;
mod dom;
mod gui;
mod html;
mod layout;
mod logic;
mod markdown;
mod resource_manager;
mod style;
mod stylednode;
mod tab;

use gui::Command;

use sdl2::messagebox::{show_message_box, ButtonData, MessageBoxButtonFlag, MessageBoxFlag};
use sdl2::mouse::SystemCursor;
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");

fn main() {
    let mut url = String::new();

    if let Some(arg) = env::args().nth(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!(
                    "
-v, --version           Print version number
dev.dev                 load local dev file
view-source:<URL>       View source code of website"
                );
                return;
            }
            "-v" | "--version" => {
                println!("{}", NAME);
                println!("Version v{}", VERSION);
                return;
            }
            _ => {
                url = arg;
            }
        }
    }

    if !Path::new("cache/").exists() {
        fs::create_dir("cache").expect("to create cache directory");
        fs::File::create("cache/cache.csv").expect("to create cache index file");
    }

    let (ttf_context, mut canvas, mut event_pump, sdl_context, text_util) =
        gui::init().expect("gui init to succed");

    let texture_creator = canvas.texture_creator();
    let mut texture_manager = resource_manager::TextureManager::new(&texture_creator);
    let mut font_manager = resource_manager::FontManager::new(&ttf_context);

    let managers = &mut (&mut texture_manager, &mut font_manager);

    // display ui
    gui::display((&mut canvas, &texture_creator), managers, &Vec::new(), 0);

    let dimensions = canvas.viewport().size();
    let mut tabs = vec![tab::Tab::new()];
    let mut current = 0;

    // holds current cursor, as it apparently needs to stay in scope to be effective
    let mut cursor;

    cursor = sdl2::mouse::Cursor::from_system(SystemCursor::WaitArrow).unwrap();
    cursor.set();
    tabs[current].browse(url, dimensions);

    cursor = sdl2::mouse::Cursor::from_system(SystemCursor::Arrow).unwrap();
    cursor.set();

    let mut window = canvas.window_mut();

    set_title(window, &tabs[current].title);

    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);

    let mut text_input = String::new();

    'running: loop {
        let (commands, text) = gui::handle_events(&mut event_pump, &sdl_context);

        let viewport = canvas.viewport();

        // FIXME: to get smooth scrolling, maybe save a offset to scroll and each frame update it by 1px until the offset is gone
        for command in &commands {
            match command {
                Command::Quit => {
                    if tabs.len() > 1 {
                        // Maybe add checkbox: Warn me when I attempt to close multiple tabs
                        let buttons: Vec<_> = vec![
                            ButtonData {
                                flags: MessageBoxButtonFlag::RETURNKEY_DEFAULT,
                                button_id: 1,
                                text: "Close Tabs",
                            },
                            ButtonData {
                                flags: MessageBoxButtonFlag::ESCAPEKEY_DEFAULT,
                                button_id: 2,
                                text: "Cancel",
                            },
                        ];
                        let res = show_message_box(
                            MessageBoxFlag::WARNING,
                            buttons.as_slice(),
                            "Close tabs?",
                            &format!("You are about to close {} tabs. Are you sure you want to continue?", tabs.len()),
                            canvas.window(),
                            None,
                        );

                        if let Ok(sdl2::messagebox::ClickedButton::CloseButton)
                        | Ok(sdl2::messagebox::ClickedButton::CustomButton(ButtonData {
                            flags: MessageBoxButtonFlag::ESCAPEKEY_DEFAULT,
                            ..
                        })) = res
                        {
                            continue;
                        }
                    }

                    break 'running;
                }
                Command::Resize => {
                    let (width, height) = viewport.size();
                    let mut layout_height = 0.0;

                    {
                        if let Some(ref styleroot) = tabs[current].style_root {
                            // FIXME: on resize, recalculate stylesheets, some (rules) may not apply anymore
                            let layout =
                                display::layout(styleroot.to_owned(), width as f32, height as f32);

                            layout_height = layout.dimensions.margin_box().height;

                            tabs[current].display_list = display::build_display_list(&layout);
                        }
                    }

                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);

                    tabs[current].layout_height = layout_height;
                    tabs[current].scrolled = 0.0;
                }
                Command::Present => {
                    canvas.present();
                }
                Command::Redraw => {
                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                }
                Command::ScrollUp => {
                    // scroll up ↑
                    let mut y_offset = tabs[current].scrolled;

                    if y_offset > 0.0 {
                        if y_offset > 30.0 {
                            y_offset = 30.0;
                        }

                        tabs[current].scrolled -= y_offset;

                        display::scroll(&mut tabs[current].display_list, y_offset);
                        gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                    }
                }
                Command::ScrollDown => {
                    // scroll down ↓
                    let vp_height = viewport.height() as f32;
                    let mut y_offset =
                        tabs[current].layout_height - tabs[current].scrolled - vp_height;

                    if y_offset > 0.0 {
                        if y_offset > 30.0 {
                            y_offset = 30.0;
                        }

                        tabs[current].scrolled += y_offset;

                        display::scroll(&mut tabs[current].display_list, -y_offset);
                        gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                    }
                }
                Command::ScrollPageUp => {
                    // scroll page up ↑
                    let height = viewport.height() as f32;
                    let mut y_offset = -tabs[current].scrolled;

                    if y_offset > 0.0 {
                        if y_offset > height {
                            y_offset = height;
                        }

                        tabs[current].scrolled += y_offset;

                        display::scroll(&mut tabs[current].display_list, y_offset);
                        gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                    }
                }
                Command::ScrollPageDown => {
                    // scroll page down ↓
                    let height = viewport.height() as f32;
                    let mut y_offset =
                        tabs[current].layout_height + (tabs[current].scrolled - height);

                    if y_offset > 0.0 {
                        if y_offset > height {
                            y_offset = height;
                        }

                        tabs[current].scrolled -= y_offset;

                        display::scroll(&mut tabs[current].display_list, -y_offset);
                        gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                    }
                }
                Command::ScrollHome => {
                    // scroll home (up) ↑
                    let scrolled = tabs[current].scrolled;
                    {
                        display::scroll(&mut tabs[current].display_list, -scrolled);
                    }
                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);

                    tabs[current].scrolled = 0.0;
                }
                Command::ScrollEnd => {
                    // scroll end (down) ↓
                    let height = viewport.height() as f32;
                    let y_offset = tabs[current].layout_height + (tabs[current].scrolled - height);

                    if y_offset > 0.0 {
                        tabs[current].scrolled -= y_offset;

                        display::scroll(&mut tabs[current].display_list, -y_offset);
                        gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                    }
                }
                Command::NewTab => {
                    tabs.push(tab::Tab::new());
                    current = tabs.len() - 1;
                    tabs[current].history.push(String::new());

                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                }
                Command::OpenUrl(url) => {
                    let dimensions = viewport.size();

                    cursor = sdl2::mouse::Cursor::from_system(SystemCursor::WaitArrow).unwrap();
                    cursor.set();
                    tabs[current].browse(url.to_string(), dimensions);
                    cursor = sdl2::mouse::Cursor::from_system(SystemCursor::Hand).unwrap();
                    cursor.set();

                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                }
                Command::Reload(new_tab) => {
                    let dimensions = viewport.size();
                    let new_url = tabs[current].url.clone();

                    if *new_tab {
                        tabs.push(tab::Tab::new());
                        current = tabs.len() - 1;
                        tabs[current].browse(new_url, dimensions);
                    } else {
                        tabs[current].open(new_url, dimensions);
                    }

                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                }
                Command::CloseTab => {
                    tabs.remove(current);
                    if tabs.is_empty() {
                        break 'running;
                    } else {
                        current = current.saturating_sub(1);
                    }

                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                }
                Command::StartTextInput => {
                    text_util.start();
                    println!("Text input started");
                    cursor = sdl2::mouse::Cursor::from_system(SystemCursor::IBeam).unwrap();
                    cursor.set();
                }
                Command::StopTextInput => {
                    text_util.stop();
                    println!("Text input stopped");
                    cursor =
                        sdl2::mouse::Cursor::from_system(sdl2::mouse::SystemCursor::Arrow).unwrap();
                    cursor.set();
                }
                Command::OpenUrlbar => {
                    if !text_input.is_empty() {
                        let dimensions = viewport.size();

                        cursor = sdl2::mouse::Cursor::from_system(SystemCursor::WaitArrow).unwrap();
                        cursor.set();
                        tabs[current].browse(text_input, dimensions);
                        cursor = sdl2::mouse::Cursor::from_system(SystemCursor::Hand).unwrap();
                        cursor.set();

                        gui::display((&mut canvas, &texture_creator), managers, &tabs, current);

                        text_input = String::new();
                    }
                }
                Command::GoForward(_new_tab) => {
                    // FIXME: open new tab if new_tab
                    let dimensions = viewport.size();

                    tabs[current].go_forward(dimensions);
                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                }
                Command::GoBack(_new_tab) => {
                    // FIXME: open new tab if new_tab
                    let dimensions = viewport.size();

                    tabs[current].go_back(dimensions);
                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                }
                Command::Fullscreen => {
                    window = canvas.window_mut();
                    let state = window.fullscreen_state();

                    let _ = if let sdl2::video::FullscreenType::Off = state {
                        window.set_fullscreen(sdl2::video::FullscreenType::Desktop)
                    } else {
                        window.set_fullscreen(sdl2::video::FullscreenType::Off)
                    };
                }
                Command::Scroll(direction) => {
                    // scroll tabs
                    match direction.cmp(&0) {
                        Ordering::Greater => {
                            // scroll up ↑
                            if current == 0 {
                                current = tabs.len();
                            }
                            current -= 1;
                        }
                        Ordering::Less => {
                            // scroll down ↓
                            current += 1;
                            current %= tabs.len();
                        }
                        Ordering::Equal => {}
                    }
                    gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                }
                Command::Click(x, y, btn) => {
                    if y < &21 {
                        // tabs

                        // FIXME: should be same as in gui.rs
                        let tab_width = 200.0;
                        let mut found = false;
                        for i in 0..tabs.len() {
                            let tab_start = i as f32 * (tab_width + 2.0);

                            if x > &((tab_start + tab_width - 20.0) as i32)
                                && x < &((tab_start + tab_width - 4.0) as i32)
                            {
                                // close on pressing X
                                tabs.remove(i);
                                if tabs.is_empty() {
                                    break 'running;
                                } else {
                                    current = current.saturating_sub(1);
                                }
                                gui::display(
                                    (&mut canvas, &texture_creator),
                                    managers,
                                    &tabs,
                                    current,
                                );
                                found = true;
                                break;
                            } else if x > &(tab_start as i32)
                                && x < &((tab_start + tab_width) as i32)
                            {
                                if btn == &sdl2::mouse::MouseButton::Left {
                                    current = i;
                                } else if btn == &sdl2::mouse::MouseButton::Middle {
                                    // close middle clicked tab
                                    tabs.remove(i);
                                    if tabs.is_empty() {
                                        break 'running;
                                    } else {
                                        current = current.saturating_sub(1);
                                    }
                                }
                                gui::display(
                                    (&mut canvas, &texture_creator),
                                    managers,
                                    &tabs,
                                    current,
                                );
                                found = true;
                                break;
                            }
                        }

                        // FIXME: same code as in Command::NewTab
                        if !found && btn == &sdl2::mouse::MouseButton::Middle {
                            // new tab
                            tabs.push(tab::Tab::new());
                            current = tabs.len() - 1;
                            tabs[current].history.push(String::new());

                            gui::display((&mut canvas, &texture_creator), managers, &tabs, current);
                        }
                    } else {
                        // browser window
                        if let Some(layout) = &tabs[current].layout {
                            // y - UI_height
                            if let Some(lbox) = layout.find_coordinate_element(
                                *x,
                                *y - 50 + tabs[current].scrolled as i32,
                            ) {
                                if let layout::BoxType::BlockNode(node)
                                | layout::BoxType::InlineNode(node, _) = &lbox.box_type
                                {
                                    if let dom::NodeType::Element(element) = &node.node.node_type {
                                        if element.tag_name == "a" {
                                            if let Some(href) = &element.get_attribute("href") {
                                                let dimensions = viewport.size();

                                                let url = (*logic::absolute_path(
                                                    &tabs[current].url,
                                                    href,
                                                ))
                                                .to_string();

                                                if btn == &sdl2::mouse::MouseButton::Middle {
                                                    // always open in new tab on middle click
                                                    tabs.push(tab::Tab::new());
                                                    current = tabs.len() - 1;
                                                } else if let Some(target) =
                                                    &element.get_attribute("target")
                                                {
                                                    if *target == "_blank" {
                                                        tabs.push(tab::Tab::new());
                                                        current = tabs.len() - 1;
                                                    }
                                                }

                                                cursor = sdl2::mouse::Cursor::from_system(
                                                    SystemCursor::WaitArrow,
                                                )
                                                .unwrap();
                                                cursor.set();
                                                tabs[current].browse(url, dimensions);
                                                cursor = sdl2::mouse::Cursor::from_system(
                                                    SystemCursor::Hand,
                                                )
                                                .unwrap();
                                                cursor.set();
                                                gui::display(
                                                    (&mut canvas, &texture_creator),
                                                    managers,
                                                    &tabs,
                                                    current,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // display text input of search bar
        if !text.is_empty() {
            text_input.push_str(&text);

            use crate::css::Color;
            use crate::display::DisplayCommand;
            use crate::layout::Rect;
            use sdl2::rect::Rect as Sdl_rect;

            let mut ui_list = Vec::new();
            ui_list.push(DisplayCommand::SolidColor(
                Color {
                    r: 71,
                    g: 71,
                    b: 73,
                    a: 255,
                },
                Rect {
                    x: 100.0,
                    y: 25.0,
                    width: (viewport.width() - 200) as f32,
                    height: 21.0,
                },
            ));

            ui_list.push(DisplayCommand::Text(
                Color {
                    r: 200,
                    g: 200,
                    b: 200,
                    a: 255,
                },
                text_input.clone(),
                Rect {
                    x: 110.0,
                    y: 24.0,
                    width: 600.0,
                    height: 16.0,
                },
                Vec::new(),
                16,
                String::new(),
            ));

            canvas.set_viewport(Sdl_rect::new(0, 0, viewport.width(), viewport.height()));

            gui::paint(
                (&mut canvas, &texture_creator),
                (managers.0, managers.1),
                &ui_list,
            )
            .expect("Couldn't paint");
            canvas.present();
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000_u32 / 60));

        if !commands.is_empty() {
            window = canvas.window_mut();
            set_title(window, &tabs[current].title);
        }
    }
}

fn set_title(window: &mut sdl2::video::Window, title: &Option<String>) {
    let window_title = if let Some(title) = title {
        let mut t = title.clone();
        t.push_str(" - prowser");
        t
    } else {
        String::from("prowser")
    };

    window
        .set_title(&window_title)
        .expect("Couldn't set title.");
}
