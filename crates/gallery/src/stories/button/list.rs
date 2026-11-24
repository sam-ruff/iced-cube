use iced::widget::column;
use iced::{Alignment, Element, Length};
use iced_cube::icon::Glyph;
use iced_cube::primitives::button::Variant;
use iced_cube::{button, lucide};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    General,
    Appearance,
    Notifications,
    Privacy,
}

impl Category {
    const ALL: [Category; 4] = [
        Category::General,
        Category::Appearance,
        Category::Notifications,
        Category::Privacy,
    ];

    fn label(self) -> &'static str {
        match self {
            Category::General => "General",
            Category::Appearance => "Appearance",
            Category::Notifications => "Notifications",
            Category::Privacy => "Privacy and security",
        }
    }

    fn icon(self) -> Glyph {
        match self {
            Category::General => lucide!(Settings),
            Category::Appearance => lucide!(Palette),
            Category::Notifications => lucide!(Bell),
            Category::Privacy => lucide!(Lock),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Selected(Category),
}

#[derive(Debug)]
pub struct Example {
    selected: Category,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            selected: Category::General,
        }
    }
}

impl Example {
    pub fn update(&mut self, message: Message) {
        let Message::Selected(category) = message;
        self.selected = category;
    }

    pub fn view(&self) -> Element<'_, Message> {
        let items = Category::ALL.map(|category| {
            let variant = if category == self.selected {
                Variant::Secondary
            } else {
                Variant::Ghost
            };
            button(category.label())
                .icon(category.icon())
                .variant(variant)
                .width(Length::Fill)
                .align_x(Alignment::Start)
                .on_press(Message::Selected(category))
                .into()
        });

        column(items).spacing(4).width(240).into()
    }
}
