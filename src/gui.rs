use crate::css::Color;
use crate::display::DisplayCommand;
use crate::layout::Rect;
use crate::resource_manager;
use crate::tab;

use sdl2::event::{Event, WindowEvent};
use sdl2::image::LoadSurface;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color as Sdl_color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect as Sdl_rect;
use sdl2::render::TextureQuery;
use sdl2::surface::Surface;
use sdl2::ttf::FontStyle;
use std::cmp::Ordering;

/// Command represents the possible actions that could result from an event
pub enum Command {
    CloseTab,
    NewTab,
    OpenUrlbar,
    OpenUrl(String),
    Present,
    Quit,
    Redraw,
    Reload(bool),
    Resize,
    ScrollDown,
    ScrollEnd,
    ScrollHome,
    ScrollPageDown,
    ScrollPageUp,
    ScrollUp,
    StartTextInput,
    StopTextInput,
    GoBack(bool),
    GoForward(bool),
    Click(i32, i32, sdl2::mouse::MouseButton),
    Scroll(i32),
    Fullscreen,
}

/// Inits sdl2
pub fn init() -> Result<
    (
        sdl2::ttf::Sdl2TtfContext,
        sdl2::render::Canvas<sdl2::video::Window>,
        sdl2::EventPump,
        sdl2::Sdl,
        sdl2::keyboard::TextInputUtil,
    ),
    String,
> {
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let text_util = video_subsystem.text_input();

    // FIXME: window should restore to dimensions and position of last time
    // https://stackoverflow.com/questions/41745492/sdl2-how-to-position-a-window-on-a-second-monitor
    let mut window = video_subsystem
        .window("prowser", 800, 600)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let window_icon = Surface::from_file("assets/icon.png")?;
    window.set_icon(window_icon);
    let _ = window.set_minimum_size(450, 124);

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    /* Optionen:
    https://wiki.libsdl.org/SDL_RendererFlags?highlight=%28%5CbCategoryEnum%5Cb%29%7C%28SDLEnumTemplate%29
    .software()
    .accelerated() // lets us use the graphics card to render quickly.
    .present_vsync()
    */
    let event_pump = sdl_context.event_pump()?;

    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);

    canvas.set_draw_color(Sdl_color::RGB(255, 255, 255));
    canvas.clear();
    canvas.present();

    Ok((ttf_context, canvas, event_pump, sdl_context, text_util))
}

/// sdl eventloop
/// [event](https://rust-sdl2.github.io/rust-sdl2/sdl2/event/enum.Event.html)
// Event::KeyDown { repeat: false, .. } to trigger only once on keydown
pub fn handle_events(
    event_pump: &mut sdl2::EventPump,
    sdl_context: &sdl2::Sdl,
) -> (Vec<Command>, String) {
    let mut commands = Vec::new();
    // https://docs.rs/sdl2/0.32.2/src/sdl2/keyboard/mod.rs.html#13
    // http://headerphile.com/sdl2/sdl2-part-11-text-styling/
    let mod_state = &sdl_context.keyboard().mod_state().bits();
    let mut text_input = String::new();
    let mouse_y = sdl2::mouse::MouseState::new(event_pump).y();

    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => {
                commands.push(Command::Quit);
            }
            Event::Window {
                win_event: WindowEvent::Resized(..),
                ..
            } => {
                commands.push(Command::Resize);
            }
            Event::Window {
                win_event: WindowEvent::Exposed,
                ..
            } => {
                commands.push(Command::Redraw);
            }
            Event::Window {
                win_event: WindowEvent::Moved(..),
                ..
            } => {
                commands.push(Command::Present);
            }
            Event::DropText { filename, .. } => {
                // TODO: search for text which is dropped. (probably getting each line by itself)
                dbg!(filename);
            }
            Event::DropFile { filename, .. } => {
                // TODO: open file which is dropped. (probably getting each file by itself)
                dbg!(filename);
            }
            Event::KeyDown {
                keycode: Some(key), ..
            } => match key {
                Keycode::PageDown => commands.push(Command::ScrollPageDown),
                Keycode::PageUp => commands.push(Command::ScrollPageUp),
                Keycode::Home => commands.push(Command::ScrollHome),
                Keycode::End => commands.push(Command::ScrollEnd),
                Keycode::Down => commands.push(Command::ScrollDown),
                Keycode::Up => commands.push(Command::ScrollUp),
                Keycode::F5 => commands.push(Command::Reload(false)),
                Keycode::F11 => commands.push(Command::Fullscreen),
                Keycode::T => {
                    let flag_ctrl = mod_state & 0x0040;
                    if flag_ctrl == 64 {
                        commands.push(Command::NewTab);
                    }
                }
                Keycode::L => {
                    let flag_ctrl = mod_state & 0x0040;
                    if flag_ctrl == 64 {
                        commands.push(Command::StartTextInput);
                    }
                }
                Keycode::W => {
                    let flag_ctrl = mod_state & 0x0040;
                    let flag_shift = mod_state & 0x0001;

                    match (flag_ctrl, flag_shift) {
                        (64, 0) => {
                            commands.push(Command::CloseTab);
                        }
                        (64, 1) => {
                            commands.push(Command::Quit);
                        }
                        _ => {}
                    }
                }
                Keycode::Return | Keycode::KpEnter => {
                    commands.push(Command::StopTextInput);
                    commands.push(Command::OpenUrlbar);
                }
                _ => {}
            },
            Event::MouseWheel { y, .. } => {
                // FIXME: maybe give all scroll info in command back and not splitt like this
                if mouse_y < 23 {
                    // scrolling in tab bar
                    commands.push(Command::Scroll(y));
                } else if mouse_y > 50 {
                    // scrolling in view
                    match y.cmp(&0) {
                        Ordering::Greater => {
                            // scroll up ↑
                            commands.push(Command::ScrollUp);
                        }
                        Ordering::Less => {
                            // scroll down ↓
                            commands.push(Command::ScrollDown);
                        }
                        Ordering::Equal => {}
                    }
                }
            }
            Event::MouseButtonDown {
                x, y, mouse_btn, ..
            } => {
                if x < 100 || x > 700 || y < 24 || y > 40 {
                    commands.push(Command::StopTextInput);
                }
                match mouse_btn {
                    sdl2::mouse::MouseButton::X1 => {
                        commands.push(Command::GoBack(false));
                    }
                    sdl2::mouse::MouseButton::X2 => {
                        commands.push(Command::GoForward(false));
                    }
                    /* sdl2::mouse::MouseButton::Right => {
                        // TODO: open right click menu
                    } */
                    _ => {
                        // ui bar
                        if y > 24 && y < 40 {
                            if x > 100 && x < 700 {
                                commands.push(Command::StartTextInput);
                            } else if x < 16 {
                                if mouse_btn == sdl2::mouse::MouseButton::Left {
                                    commands.push(Command::GoBack(false));
                                } else if mouse_btn == sdl2::mouse::MouseButton::Middle {
                                    commands.push(Command::GoBack(true));
                                }
                            } else if x > 18 && x < 34 {
                                if mouse_btn == sdl2::mouse::MouseButton::Left {
                                    commands.push(Command::GoForward(false));
                                } else if mouse_btn == sdl2::mouse::MouseButton::Middle {
                                    commands.push(Command::GoForward(true));
                                }
                            } else if x > 36 && x < 52 {
                                // reload button
                                if mouse_btn == sdl2::mouse::MouseButton::Left {
                                    commands.push(Command::Reload(false));
                                } else if mouse_btn == sdl2::mouse::MouseButton::Middle {
                                    commands.push(Command::Reload(true));
                                }
                            } else if x > 54 && x < 70 {
                                // home button
                                /*
                                    either use the OpenUrl command to open "home" in the default tab.
                                    Or reuse the "newtab" command but in the same tab. aka a tab reset
                                */
                                if mouse_btn == sdl2::mouse::MouseButton::Left {
                                    // FIXME: home button should bring you to the specified url, not always the empty tab
                                    commands.push(Command::OpenUrl(String::new()));
                                }
                            }
                        } else if y >= 50 || y < 24 {
                            // browser window or tabs
                            commands.push(Command::Click(x, y, mouse_btn));
                        }
                    }
                }
            }
            Event::TextInput { text, .. } => {
                text_input.push_str(&text);
            }
            _ => {}
        }
    }

    (commands, text_input)
}

/// Clear, paint UI, paint Page
pub fn display(
    gui: (
        &mut sdl2::render::Canvas<sdl2::video::Window>,
        &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    ),
    managers: &mut (
        &mut resource_manager::TextureManager<sdl2::video::WindowContext>,
        &mut resource_manager::FontManager,
    ),
    tabs: &[tab::Tab],
    current_tab: usize,
) {
    let canvas = gui.0;
    let texture_creator = gui.1;

    let (width, height) = canvas.window().size();

    canvas.set_draw_color(Sdl_color::RGB(255, 255, 255));
    canvas.clear();
    canvas.set_viewport(Sdl_rect::new(0, 0, width, height));

    // ui
    let mut ui_list = Vec::new();

    // top nav bar
    // black bar
    ui_list.push(DisplayCommand::SolidColor(
        Color {
            r: 12,
            g: 12,
            b: 13,
            a: 255,
        },
        Rect {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: 22.0,
        },
    ));

    // tabs
    let tab_width = 200.0;
    for (i, tab) in tabs.iter().enumerate() {
        // highlight current tab
        let color = if i as usize == current_tab {
            Color {
                r: 125,
                g: 125,
                b: 125,
                a: 255,
            }
        } else {
            Color {
                r: 75,
                g: 75,
                b: 75,
                a: 255,
            }
        };
        ui_list.push(DisplayCommand::SolidColor(
            color,
            Rect {
                x: i as f32 * (tab_width + 2.0),
                y: 1.0,
                width: tab_width,
                height: 21.0,
            },
        ));

        let mut favicon = 0.0;
        let mut max_title_length = 22;
        // favicon
        if let Some(path) = &tab.favicon {
            favicon = 18.0;
            max_title_length -= 3;
            ui_list.push(DisplayCommand::Image(
                path.to_string(),
                Rect {
                    x: 5.0 + i as f32 * (tab_width + 2.0),
                    y: 2.0,
                    width: 16.0,
                    height: 16.0,
                },
            ));
        }

        let tab_title = if let Some(title) = &tab.title {
            if title.len() < max_title_length {
                title.clone()
            } else {
                title[..max_title_length].to_string()
            }
        } else if tab.url.len() < max_title_length {
            tab.url.clone()
        } else {
            tab.url[..max_title_length].to_string()
        };
        ui_list.push(DisplayCommand::Text(
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            tab_title,
            Rect {
                x: 5.0 + i as f32 * (tab_width + 2.0) + favicon,
                y: 2.0,
                width: tab_width,
                height: 21.0,
            },
            Vec::new(),
            16,
            String::new(),
        ));
        ui_list.push(DisplayCommand::Text(
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            String::from("X"),
            Rect {
                x: 5.0 + i as f32 * (tab_width + 2.0) + tab_width - 20.0,
                y: 4.0,
                width: 16.0,
                height: 16.0,
            },
            vec![String::from("bold")],
            14,
            String::new(),
        ));

        // theme color
        if let Some(theme_color) = &tab.color {
            ui_list.push(DisplayCommand::SolidColor(
                theme_color.clone(),
                Rect {
                    x: i as f32 * (tab_width + 2.0),
                    y: 1.0,
                    width: tab_width,
                    height: 2.0,
                },
            ));
        }
    }

    // dark gray background
    ui_list.push(DisplayCommand::SolidColor(
        Color {
            r: 50,
            g: 50,
            b: 52,
            a: 255,
        },
        Rect {
            x: 0.0,
            y: 22.0,
            width: width as f32,
            height: 29.0,
        },
    ));

    // buttons
    ui_list.push(DisplayCommand::Image(
        String::from("assets/right.png"),
        Rect {
            x: 18.0,
            y: 28.0,
            width: 16.0,
            height: 16.0,
        },
    ));
    ui_list.push(DisplayCommand::Image(
        String::from("assets/left.png"),
        Rect {
            x: 0.0,
            y: 28.0,
            width: 16.0,
            height: 16.0,
        },
    ));
    ui_list.push(DisplayCommand::Image(
        String::from("assets/reload.png"),
        Rect {
            x: 36.0,
            y: 28.0,
            width: 16.0,
            height: 16.0,
        },
    ));
    ui_list.push(DisplayCommand::Image(
        String::from("assets/home.png"),
        Rect {
            x: 54.0,
            y: 28.0,
            width: 16.0,
            height: 16.0,
        },
    ));

    // url bar
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
            width: (width - 200) as f32,
            height: 21.0,
        },
    ));
    if !tabs.is_empty() && !tabs[current_tab].url.is_empty() {
        ui_list.push(DisplayCommand::Text(
            Color {
                r: 200,
                g: 200,
                b: 200,
                a: 255,
            },
            tabs[current_tab].url.clone(),
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
    }

    paint(
        (canvas, texture_creator),
        (managers.0, managers.1),
        &ui_list,
    )
    .expect("Couldn't paint");

    canvas.set_viewport(Sdl_rect::new(0, 51, width, height - 51));
    if !tabs.is_empty() {
        paint(
            (canvas, texture_creator),
            (managers.0, managers.1),
            &tabs[current_tab].display_list,
        )
        .expect("Couldn't paint");
    }

    canvas.present();
}

/// Paint a tree of `LayoutBoxes` on the gui.
fn paint(
    gui: (
        &mut sdl2::render::Canvas<sdl2::video::Window>,
        &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    ),
    managers: (
        &mut resource_manager::TextureManager<sdl2::video::WindowContext>,
        &mut resource_manager::FontManager,
    ),
    display_list: &[DisplayCommand],
) -> Result<(), String> {
    // FIXME: is this line needed?
    //sdl2::image::init(InitFlag::PNG | InitFlag::JPG | InitFlag::TIF | InitFlag::WEBP)?;

    let texture_manager = managers.0;
    let font_manager = managers.1;

    let canvas = gui.0;
    let texture_creator = gui.1;

    let viewport = canvas.viewport();
    let width = viewport.width() as f32;
    let height = viewport.height() as f32;

    for item in display_list {
        match item {
            DisplayCommand::SolidColor(_, rect)
            | DisplayCommand::Text(_, _, rect, ..)
            | DisplayCommand::Image(_, rect)
            | DisplayCommand::Gradient(rect, ..) => {
                if rect.y > height || rect.x > width {
                    // early break for offscreen elements
                    break;
                } else if rect.y + rect.height < 0.0 || rect.x + rect.width < 0.0 {
                    // skip elements which are over the viewport
                    continue;
                }
            }
        }

        match item {
            DisplayCommand::SolidColor(color, rect) => {
                let target = rect.to_sdlrect();
                canvas.set_draw_color(Sdl_color::RGBA(color.r, color.g, color.b, color.a));
                canvas.fill_rect(target)?;
            }
            DisplayCommand::Text(foreground, text, rect, styles, size, family) => {
                // render a surface, and convert it to a texture bound to the canvas

                let font_to_load = if family == "serif" {
                    "assets/bitstream-vera-1.10/VeraSe.ttf"
                } else if family == "monospace" {
                    "assets/bitstream-vera-1.10/VeraMono.ttf"
                } else {
                    "assets/bitstream-vera-1.10/Vera.ttf"
                };

                // Set font styles http://headerphile.com/sdl2/sdl2-part-11-text-styling/
                let mut font_style = 0;
                for style in styles {
                    font_style |= match &**style {
                        "underline" => FontStyle::UNDERLINE,
                        "line-through" => FontStyle::STRIKETHROUGH,
                        "bold" => FontStyle::BOLD,
                        "italic" => FontStyle::ITALIC,
                        _ => FontStyle::NORMAL,
                    }
                    .bits();
                }
                let style = FontStyle::from_bits_truncate(font_style);

                let font = font_manager.load(&resource_manager::FontDetails {
                    path: font_to_load.to_string(),
                    size: *size,
                    style,
                })?;

                /*
                solid, shaded, blended ; fastest to slowest
                As you can see, both the arguments and return value is the same for TTF_RenderText_Solid and TTF_RenderText_Blended. So what’s the difference between TTF_RenderText_Solid and TTF_RenderText_Blended? The difference is that TTF_RenderText_Solid is very quick, but TTF_RenderText_Blended produces a better result. In our game, we won’t be updating our text surfaces all that often, and there’s not a lot of them either, so TTF_RenderText_Blended is a good choice.
                */
                let surface = font
                    .render(text)
                    .blended(Sdl_color::RGBA(
                        foreground.r,
                        foreground.g,
                        foreground.b,
                        foreground.a,
                    ))
                    .map_err(|e| e.to_string())?;

                // if texture creator throws error, like "Texture dimensions are limited to 8192x8192", then just skip it
                let texture = match texture_creator.create_texture_from_surface(&surface) {
                    Err(_) => {
                        continue;
                    }
                    Ok(ok) => ok,
                };

                let TextureQuery { width, height, .. } = texture.query();

                let target = Sdl_rect::new(rect.x as i32, rect.y as i32, width, height);

                canvas.copy(&texture, None, Some(target))?;
            }
            DisplayCommand::Image(path, rect) => {
                let target = rect.to_sdlrect();

                // if texture creator throws error, like "Unsupported image format", then just skip it
                // FIXME: show placeholder instead
                let texture = match texture_manager.load(path) {
                    Err(_) => {
                        continue;
                    }
                    Ok(ok) => ok,
                };

                canvas.copy(&texture, None, Some(target))?;
            }
            DisplayCommand::Gradient(rect, _direction, _colors) => {
                let target = rect.to_sdlrect();

                let mut texture = texture_creator
                    .create_texture_streaming(PixelFormatEnum::RGB24, 256, 256) // width, height
                    .map_err(|e| e.to_string())?;
                // Create a red-green gradient
                texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                    for y in 0..256 {
                        for x in 0..256 {
                            let offset = y * pitch + x * 3;
                            buffer[offset] = x as u8;
                            buffer[offset + 1] = y as u8;
                            buffer[offset + 2] = 0;
                        }
                    }
                })?;
                canvas.copy(&texture, None, Some(target))?;

                /* https://github.com/Rust-SDL2/rust-sdl2/blob/master/examples/no-renderer.rs
                                fn set_window_gradient(window: &mut Window, event_pump: &sdl2::EventPump, gradient: Gradient) -> Result<(), String> {
                    let mut surface = window.surface(event_pump)?;
                    for i in 0 .. (WINDOW_WIDTH / 4) {
                        let c : u8 = 255 - (i as u8);
                        let i = i as i32;
                        let color = match gradient {
                            Gradient::Red => Color::RGB(c, 0, 0),
                            Gradient::Cyan => Color::RGB(0, c, c),
                            Gradient::Green => Color::RGB(0, c, 0),
                            Gradient::Blue => Color::RGB(0, 0, c),
                            Gradient::White => Color::RGB(c, c, c),
                        };
                        surface.fill_rect(Rect::new(i*4, 0, 4, WINDOW_HEIGHT), color)?;
                    }
                    surface.finish()
                }
                */
            }
        }
    }
    Ok(())
}

/*
// for 30 grad angle.
// last two are to flip

canvas.copy_ex(
        &texture,
        None,
        Some(Rect::new(450, 100, 256, 256)),
        30.0,
        None,
        false,
        false,
    )?;

*/
