mod paths;

use iced::{Element, Theme};

fn main() -> iced::Result {
    iced::application("vrc-ytdlp", App::update, App::view)
        .theme(|_| Theme::Dark)
        .run()
}

#[derive(Debug, Default)]
struct App {}

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn update(&mut self, _message: Message) {}

    fn view(&self) -> Element<Message> {
        iced::widget::text("vrc-ytdlp GUI").into()
    }
}
