use iced::{
    Element,
    Length::{self, Fill},
    Task, widget,
};
use parser::{Kyu, QuestionField};

#[derive(Debug, Clone)]
struct KankenBrowser {
    questions: Vec<QuestionField>,
    selected_kyu: Kyu,
    selected_mode: String,
    current_question_index: usize,
}
#[derive(Debug, Clone)]
enum Message {
    SelectKyu(Kyu),
    SelectMode(String),
    NextQuestion,
    PrevQuestion,
}
impl KankenBrowser {
    fn new() -> (Self, Task<Message>) {
        let content = std::fs::read_to_string("data/combined_fields.json").unwrap();
        let questions: Vec<QuestionField> = serde_json::from_str(&content).unwrap();
        let selected_kyu = Kyu::Kyu10;
        let selected_mode = questions
            .iter()
            .find(|f| f.field_info.level == selected_kyu)
            .map(|f| f.field_info.name.clone())
            .unwrap_or("読み".into());

        (
            Self {
                questions,
                selected_kyu,
                selected_mode,
                current_question_index: 0,
            },
            Task::none(),
        )
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::SelectKyu(kyu) => {
                self.selected_kyu = kyu;
                self.selected_mode = self
                    .questions
                    .iter()
                    .find(|f| f.field_info.level == kyu)
                    .map(|f| f.field_info.name.clone())
                    .unwrap_or("読み".into());
                self.current_question_index = 0;
            }
            Message::SelectMode(mode) => {
                self.selected_mode = mode;
                self.current_question_index = 0;
            }
            Message::NextQuestion => {
                let len = self
                    .questions
                    .iter()
                    .find(|f| {
                        f.field_info.level == self.selected_kyu
                            && f.field_info.name == self.selected_mode
                    })
                    .map(|f| f.items.len())
                    .unwrap_or(0);
                if self.current_question_index + 1 < len {
                    self.current_question_index += 1;
                }
            }
            Message::PrevQuestion => {
                if self.current_question_index > 0 {
                    self.current_question_index -= 1;
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let kyu_buttons = Kyu::ALL.iter().map(|&kyu| {
            widget::button(widget::text(kyu.label()).center())
                .on_press(Message::SelectKyu(kyu))
                .width(iced::Fill)
                .into()
        });

        let kyu_bar = widget::scrollable(
            widget::container(widget::row(kyu_buttons).spacing(6).width(iced::Fill))
                .padding(8)
                .width(iced::Fill),
        );

        let mode_buttons = self
            .questions
            .iter()
            .filter(|f| f.field_info.level == self.selected_kyu)
            .map(|f| {
                widget::button(widget::text(f.field_info.name.as_str()).center())
                    .on_press(Message::SelectMode(f.field_info.name.clone()))
                    .width(Fill)
                    .into()
            });

        let sidebar = widget::container(widget::column(mode_buttons).spacing(6).width(200))
            .padding(8)
            .center_y(Fill);

        let content: Element<_> = match self.questions.iter().find(|f| {
            f.field_info.level == self.selected_kyu && &f.field_info.name == &self.selected_mode
        }) {
            None => widget::text("not found").into(),
            Some(field) => {
                widget::container(Self::view_question(field, self.current_question_index))
                    .center(Fill)
                    .into()
            }
        };
        let total = self
            .questions
            .iter()
            .find(|f| {
                f.field_info.level == self.selected_kyu && f.field_info.name == self.selected_mode
            })
            .map(|f| f.items.len())
            .unwrap_or(0);

        let nav = widget::container(
            widget::row![
                widget::space::horizontal(),
                widget::button(widget::text("前").center())
                    .on_press(Message::PrevQuestion)
                    .width(80),
                widget::space::horizontal(),
                widget::text(format!("{}/{}", self.current_question_index + 1, total)),
                widget::space::horizontal(),
                widget::button(widget::text("次").center())
                    .width(80)
                    .on_press(Message::NextQuestion),
                widget::space::horizontal(),
            ]
            .padding(8)
            .spacing(8)
            .padding(8),
        )
        .width(Fill)
        .center_x(Fill);

        let field_info_panel = match self.questions.iter().find(|f| {
            f.field_info.level == self.selected_kyu && f.field_info.name == self.selected_mode
        }) {
            None => widget::column![].into(),
            Some(field) => Self::view_field_info(field, self.current_question_index),
        };

        widget::column![
            kyu_bar,
            widget::row![
                widget::column![sidebar, field_info_panel],
                widget::column![content, nav]
            ]
            .height(Fill)
        ]
        .into()
    }
    fn view_question<'a>(field: &'a QuestionField, index: usize) -> Element<'a, Message> {
        let q = &field.items[index];
        // choice-type question (e.g. 正しい読み方)
        let choices = if !q.answer_choices.is_empty() {
            let options: Vec<&str> = q.answer_choices.split(' ').collect();
            let choice_buttons = widget::column(options.iter().map(|opt| {
                widget::button(widget::text(*opt).center())
                    .width(Fill)
                    .into()
            }))
            .spacing(8)
            .width(300);

            Some(choice_buttons)
        } else {
            None
        };

        let mut spans: Vec<widget::text::Span<'a, Message>> = Vec::new();
        let mut remaining = q.sentence.as_str();
        let mut bracket_index = 0;

        while let Some(open) = remaining.find('｛') {
            if open > 0 {
                spans.push(widget::span(&remaining[..open]));
            }
            remaining = &remaining[open + '｛'.len_utf8()..];
            if let Some(close) = remaining.find('｝') {
                let word = &remaining[..close];
                if bracket_index == q.selected_index {
                    spans.push(widget::span(word).color(iced::color!(0xff, 0x44, 0x44)));
                } else {
                    spans.push(widget::span(word));
                }
                remaining = &remaining[close + '｝'.len_utf8()..];
                bracket_index += 1;
            }
        }
        if !remaining.is_empty() {
            spans.push(widget::span(remaining));
        }

        if let Some(choices) = choices {
            widget::column![widget::rich_text(spans).size(20), choices]
                .spacing(16)
                .align_x(iced::Center)
                .into()
        } else {
            widget::rich_text(spans).size(20).into()
        }
    }
    fn view_field_info<'a>(field: &'a QuestionField, index: usize) -> Element<'a, Message> {
        let info = &field.field_info;
        let q = &field.items[index];
        let score = match info.allocation_score {
            parser::AllocationScore::OnePoint => "1点",
            parser::AllocationScore::TwoPoints => "2点",
        };

        let answer_type = match info.answer_type {
            parser::AnswerType::Handwriting => "手書き",
            parser::AnswerType::Number => "数字",
            parser::AnswerType::Choice => "選択",
        };

        let answers = q.correct_answer_list.join("、");
        let rows = widget::container(
            widget::column![
                Self::row_info(info.level.label(), info.name.clone()),
                widget::container(widget::rule::horizontal(1.0)).max_width(200.),
                widget::text(&info.preamble).size(12),
                widget::container(widget::rule::horizontal(1.0)).max_width(200.),
                Self::row_info("配点", score.into()),
                Self::row_info("回答形式", answer_type.into()),
                Self::row_info("試験出題数", format!("{}問", info.count_per_exam)),
                widget::container(widget::rule::horizontal(1.0)).max_width(200.),
                Self::row_info("年度", format!("{}年", q.year)),
                Self::row_info("答え", answers),
            ]
            .spacing(4)
            .padding(8),

        ).align_bottom(Length::Shrink)
        .into();

        rows
    }

    fn row_info<'a>(label: &'a str, value: String) -> Element<'a, Message> {
        widget::row![
            widget::text(label).size(13).width(80),
            widget::space::horizontal().width(Length::Fixed(30.)),
            widget::text(value).size(13),
        ]
        .into()
    }
}

fn main() -> iced::Result {
    iced::application(
        KankenBrowser::new,
        KankenBrowser::update,
        KankenBrowser::view,
    )
    .run()
}
