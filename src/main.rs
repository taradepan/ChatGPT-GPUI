mod api;
mod message;
mod ui;

use api::client::OpenAIClient;
use gpui::*;
use ui::chat_window::ChatWindow;
use ui::input_box::*;

actions!(app, [Quit]);

const WINDOW_WIDTH: f32 = 600.0;
const WINDOW_HEIGHT: f32 = 600.0;

fn main() {
    dotenv::dotenv().ok();

    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(key) if !key.trim().is_empty() => key,
        _ => {
            eprintln!("Error: OPENAI_API_KEY not found or empty");
            eprintln!("Please set your OpenAI API key in the .env file:");
            eprintln!("  OPENAI_API_KEY=your-api-key-here");
            std::process::exit(1);
        }
    };

    let api_client = OpenAIClient::new(api_key);

    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);

        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("shift-left", SelectLeft, None),
            KeyBinding::new("shift-right", SelectRight, None),
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
            KeyBinding::new("enter", ui::chat_window::Submit, None),
            KeyBinding::new("cmd-q", Quit, None),
        ]);

        let window = match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background: WindowBackgroundAppearance::Blurred,
                titlebar: Some(TitlebarOptions {
                    appears_transparent: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ChatWindow::new(api_client.clone(), cx)),
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create window: {}", e);
                return;
            }
        };

        cx.on_keyboard_layout_change({
            move |cx| {
                let _ = window.update(cx, |_, _, cx| cx.notify());
            }
        })
        .detach();

        if let Err(e) = window.update(cx, |view, window, cx| {
            window.focus(&view.text_input.read(cx).focus_handle.clone());
            cx.activate(true);
        }) {
            eprintln!("Failed to focus window: {}", e);
        }

        cx.on_action(|_: &Quit, cx| cx.quit());
    });
}
