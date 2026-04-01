use iced::widget::text;
use iced::Element;

pub fn view<'a, M: 'a>() -> Element<'a, M> {
    text("Dashboard").into()
}
