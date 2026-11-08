//! Checks that the rich text editor draws bold and italic in their own
//! faces, with the bundled fonts so the result is the same on every machine.

use gallery::{FONT, FONT_FILES};
use iced::{Element, Size};
use iced_cube::rich_text_editor::{
    Action, Block, BlockKind, Content, Document, Mark, Run, Style, rich_text_editor,
};
use iced_test::Error;
use iced_test::simulator::Simulator;

/// A hash of the editor drawing `runs` on one line.
fn draw(runs: Vec<Run>, name: &str) -> Result<String, Error> {
    iced_cube::theme::set_font(FONT);
    let content = Content::with_document(Document::from_blocks([Block::with_runs(
        BlockKind::Paragraph,
        runs,
    )]));
    let settings = iced::Settings {
        default_font: FONT,
        fonts: FONT_FILES.iter().map(|font| (*font).into()).collect(),
        ..iced::Settings::default()
    };
    let editor: Element<'_, Action> = rich_text_editor(&content).on_action(|action| action).into();
    let mut ui = Simulator::with_size(settings, Size::new(320.0, 120.0), editor);
    let directory =
        std::env::temp_dir().join(format!("gallery-rich-text-{name}-{}", std::process::id()));
    let _ = ui
        .snapshot(&iced_cube::theme::light())?
        .matches_hash(directory.join(name))?;
    let mut hash = String::new();
    for file in std::fs::read_dir(&directory)?.filter_map(Result::ok) {
        hash = std::fs::read_to_string(file.path())?;
    }
    std::fs::remove_dir_all(&directory)?;
    Ok(hash)
}

fn styled(mark: Mark) -> Style {
    Style::default().with(mark)
}

#[test]
fn bold_and_italic_use_their_own_faces() -> Result<(), Error> {
    let plain = draw(vec![Run::plain("abcd")], "plain")?;
    let bold = draw(vec![Run::new("abcd", styled(Mark::Bold))], "bold")?;
    let italic = draw(vec![Run::new("abcd", styled(Mark::Italic))], "italic")?;
    let both = draw(
        vec![Run::new("abcd", styled(Mark::Bold).with(Mark::Italic))],
        "both",
    )?;
    assert_ne!(plain, bold);
    assert_ne!(plain, italic);
    assert_ne!(bold, italic);
    assert_ne!(both, bold);
    assert_ne!(both, italic);
    Ok(())
}

/// The shaper draws a whole word in the face of its first letter unless
/// the editor splits it, so a word bold only at its end would look plain.
#[test]
fn part_of_a_word_takes_its_own_face() -> Result<(), Error> {
    let plain = draw(vec![Run::plain("abcd")], "word-plain")?;
    let end_bold = draw(
        vec![Run::plain("ab"), Run::new("cd", styled(Mark::Bold))],
        "word-end-bold",
    )?;
    let all_bold = draw(vec![Run::new("abcd", styled(Mark::Bold))], "word-bold")?;
    let start_bold = draw(
        vec![Run::new("ab", styled(Mark::Bold)), Run::plain("cd")],
        "word-start-bold",
    )?;
    assert_ne!(end_bold, plain);
    assert_ne!(start_bold, all_bold);
    assert_ne!(start_bold, end_bold);
    let italic_end = draw(
        vec![Run::plain("ab"), Run::new("cd", styled(Mark::Italic))],
        "word-end-italic",
    )?;
    assert_ne!(italic_end, plain);
    Ok(())
}
