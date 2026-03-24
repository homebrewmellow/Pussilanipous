use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

const TEXT_FG: Color = Color::Indexed(223);

pub fn render_lines() -> Vec<Line<'static>> {
    vec![
        tinted_text("ratatui rust app"),
        tinted_text("made with Codex"),
        tinted_text("by OpenAI"),
        tinted_text("guided by"),
        tinted_text("@homebrewmellow's prompts"),
        Line::raw(""),
        tinted_text("q to close"),
    ]
}

fn tinted_text(text: &'static str) -> Line<'static> {
    Line::from(tinted_spans(text))
}

fn tinted_spans(text: &'static str) -> Vec<Span<'static>> {
    text.chars()
        .map(|ch| {
            if ch == ' ' {
                Span::raw(" ")
            } else {
                let mut cell = [0_u8; 4];
                let glyph = ch.encode_utf8(&mut cell).to_string();
                Span::styled(glyph, Style::default().fg(TEXT_FG))
            }
        })
        .collect()
}
