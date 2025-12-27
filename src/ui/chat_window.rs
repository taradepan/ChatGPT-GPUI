use gpui::prelude::*;
use gpui::*;
use std::time::Instant;

use crate::api::client::OpenAIClient;
use crate::message::{Message, Role};
use crate::ui::input_box::TextInput;
use crate::ui::theme;

actions!(chat_window, [Submit]);

const MAX_MESSAGES: usize = 200;
const DEBOUNCE_MS: u64 = 50;

pub struct ChatWindow {
    pub text_input: Entity<TextInput>,
    messages: Vec<Message>,
    focus_handle: FocusHandle,
    api_client: OpenAIClient,
    is_loading: bool,
    next_message_id: usize,
}

impl ChatWindow {
    pub fn new(api_client: OpenAIClient, cx: &mut App) -> Self {
        let text_input = cx.new(|cx| TextInput::new(cx));

        Self {
            text_input,
            messages: vec![],
            focus_handle: cx.focus_handle(),
            api_client,
            is_loading: false,
            next_message_id: 0,
        }
    }

    fn get_next_message_id(&mut self) -> usize {
        let id = self.next_message_id;
        self.next_message_id += 1;
        id
    }

    fn on_submit(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }

        let content = self.text_input.read(cx).content.trim().to_string();
        if content.is_empty() {
            return;
        }

        self.is_loading = true;

        let user_msg_id = self.get_next_message_id();
        self.messages.push(Message {
            id: user_msg_id,
            role: Role::User,
            content,
        });

        self.text_input
            .update(cx, |text_input, _cx| text_input.reset());

        let assistant_message_id = self.get_next_message_id();
        self.messages.push(Message {
            id: assistant_message_id,
            role: Role::Assistant,
            content: String::new(),
        });

        if self.messages.len() > MAX_MESSAGES {
            self.messages.drain(0..2);
        }

        let message_count = self.messages.len();
        cx.notify();

        let api_client = self.api_client.clone();
        let messages = self.messages[..message_count - 1].to_vec();
        let target_message_id = assistant_message_id;

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let (tx, rx) = std::sync::mpsc::channel();

                    let streaming_result =
                        api_client.send_message_streaming(&messages, move |token| {
                            let _ = tx.send(token);
                        });

                    (rx, streaming_result)
                })
                .await;

            let (rx, streaming_result) = result;

            let mut accumulated_content = String::with_capacity(2048);
            let mut last_update = Instant::now();

            for token in rx {
                accumulated_content.push_str(&token);

                let now = Instant::now();
                let should_update =
                    now.duration_since(last_update).as_millis() >= DEBOUNCE_MS as u128;

                if should_update {
                    last_update = now;

                    if let Err(err) = cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            if let Some(msg) =
                                this.messages.iter_mut().find(|m| m.id == target_message_id)
                            {
                                msg.content = accumulated_content.clone();
                                cx.notify();
                            }
                        })
                    }) {
                        eprintln!("Failed to update message during streaming: {:?}", err);
                        break;
                    }
                }
            }

            if let Err(err) = cx.update(|cx| {
                this.update(cx, |this, cx| {
                    if let Some(msg) = this.messages.iter_mut().find(|m| m.id == target_message_id)
                    {
                        msg.content = accumulated_content.clone();
                        cx.notify();
                    }
                })
            }) {
                eprintln!("Failed final update: {:?}", err);
            }

            if let Err(update_err) = cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.is_loading = false;

                    if let Err(err) = streaming_result {
                        if let Some(msg) =
                            this.messages.iter_mut().find(|m| m.id == target_message_id)
                        {
                            // Append error to existing content
                            if !msg.content.is_empty() {
                                msg.content.push_str("\n\n");
                            }
                            msg.content.push_str(&format!("Error: {}", err));
                        }
                    }
                    cx.notify();
                })
            }) {
                eprintln!("Failed to finalize message: {:?}", update_err);
            }

            anyhow::Ok(())
        })
        .detach();
    }

    fn on_submit_click(&mut self, _: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.on_submit(window, cx);
    }

    fn on_submit_action(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        self.on_submit(window, cx);
    }
}

impl Focusable for ChatWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ChatWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::on_submit_action))
            .flex()
            .flex_col()
            .size_full()
            .child(
                div()
                    .id("messages-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .overflow_y_scroll()
                    .pt(px(40.))
                    .px_4()
                    .pb_4()
                    .gap_2()
                    .children(self.messages.iter().map(|msg| {
                        let (bg_color, text_prefix, label_color) = match msg.role {
                            Role::User => (theme::user_message_bg(), "You", hsla(0., 0., 0.9, 1.0)),
                            Role::Assistant => {
                                (theme::assistant_message_bg(), "AI", hsla(0., 0., 0.85, 1.0))
                            }
                        };

                        let content =
                            if matches!(msg.role, Role::Assistant) && msg.content.is_empty() {
                                div()
                                    .text_color(hsla(0., 0., 0.7, 0.8))
                                    .text_size(px(15.))
                                    .child("●●●")
                            } else {
                                div()
                                    .text_color(hsla(0., 0., 0.95, 1.0))
                                    .text_size(px(15.))
                                    .line_height(px(22.))
                                    .overflow_x_hidden()
                                    .whitespace_normal()
                                    .child(msg.content.clone())
                            };

                        div()
                            .id(("message", msg.id))
                            .p_4()
                            .bg(bg_color)
                            .rounded(px(16.))
                            .border_1()
                            .border_color(theme::border_color())
                            .shadow_sm()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_size(px(13.))
                                            .text_color(label_color)
                                            .child(text_prefix),
                                    )
                                    .child(content),
                            )
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .p_4()
                    .bg(theme::input_background())
                    .border_t_1()
                    .border_color(theme::input_border())
                    .shadow_md()
                    .child(self.text_input.clone())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .h(px(44.))
                            .px_6()
                            .bg(theme::submit_button_bg())
                            .text_color(white())
                            .text_size(px(14.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .rounded(px(12.))
                            .shadow_md()
                            .child("Send")
                            .hover(|style| style.bg(theme::submit_button_hover()).cursor_pointer())
                            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_submit_click)),
                    ),
            )
    }
}
