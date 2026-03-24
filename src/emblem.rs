use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    Frame,
    widgets::Paragraph,
};

const STAGE_WIDTH: u16 = 70;
const STAGE_HEIGHT: u16 = 16;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    pulse: u32,
    reaction_ticks: u8,
    offset_x: i16,
    offset_y: i16,
    enraged: bool,
) {
    let background = ray_field(area, pulse);
    frame.render_widget(Paragraph::new(background), area);

    let stage = rendered_stage_rect(area, offset_x, offset_y);
    let emblem_shift = reaction_shift(reaction_ticks);
    let [orb_zone] = Layout::vertical([Constraint::Length(6)])
        .flex(Flex::Center)
        .areas(stage);
    let [orb_zone] = Layout::horizontal([Constraint::Length(19)])
        .flex(Flex::Center)
        .areas(orb_zone);

    if reaction_ticks > 0 || enraged {
        let halo = vec![
            Line::styled(
                "      ✦  ✦  ✦      ",
                Style::default().fg(if enraged { Color::LightRed } else { Color::Indexed(230) }),
            ),
            Line::styled(
                if enraged {
                    "    ✦  ENRAGED  ✦   "
                } else {
                    "   ✦  RADIANCE  ✦   "
                },
                Style::default().fg(if enraged { Color::Indexed(196) } else { Color::Indexed(229) }),
            ),
            Line::styled(
                "      ✦  ✦  ✦      ",
                Style::default().fg(if enraged { Color::LightRed } else { Color::Indexed(230) }),
            ),
        ];
        frame.render_widget(Paragraph::new(halo).alignment(Alignment::Center), orb_zone);
    }

    let shifted_stage = clamp_rect_to_area(
        Rect::new(
            stage.x,
            stage.y.saturating_add_signed(emblem_shift),
            stage.width,
            stage.height,
        ),
        area,
    );

    let emblem = Paragraph::new(mascot_lines(reaction_ticks, enraged))
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Reset));

    frame.render_widget(emblem, shifted_stage);
}

pub fn is_cursor_over(area: Rect, x: u16, y: u16, offset_x: i16, offset_y: i16) -> bool {
    let clickable = logical_clickable_rect(area, offset_x, offset_y);
    clickable.contains(x as i16, y as i16)
}

pub fn overlaps_cursor(area: Rect, cursor: Rect, offset_x: i16, offset_y: i16) -> bool {
    let clickable = logical_clickable_rect(area, offset_x, offset_y);
    clickable.intersects_rect(cursor)
}

pub fn center(area: Rect, offset_x: i16, offset_y: i16) -> (u16, u16) {
    let clickable = logical_clickable_rect(area, offset_x, offset_y);
    (
        clickable.center_x().clamp(0, i16::MAX) as u16,
        clickable.center_y().clamp(0, i16::MAX) as u16,
    )
}

fn logical_clickable_rect(area: Rect, offset_x: i16, offset_y: i16) -> SignedRect {
    let stage = logical_stage_origin(area, offset_x, offset_y);
    let clickable_width = 17;
    let clickable_height = 7;
    SignedRect {
        x: stage.0 + (STAGE_WIDTH.saturating_sub(clickable_width) / 2) as i16,
        y: stage.1 + 1,
        width: clickable_width as i16,
        height: clickable_height as i16,
    }
}

pub fn stage_center(area: Rect, offset_x: i16, offset_y: i16) -> (i16, i16) {
    let (x, y) = logical_stage_origin(area, offset_x, offset_y);
    (x + STAGE_WIDTH as i16 / 2, y + STAGE_HEIGHT as i16 / 2)
}

fn logical_stage_origin(area: Rect, offset_x: i16, offset_y: i16) -> (i16, i16) {
    let stage = stage_rect(area);
    (stage.x as i16 + offset_x, stage.y as i16 + offset_y)
}

fn rendered_stage_rect(area: Rect, offset_x: i16, offset_y: i16) -> Rect {
    let (x, y) = logical_stage_origin(area, offset_x, offset_y);
    clamp_rect_to_area(Rect::new(x.max(0) as u16, y.max(0) as u16, STAGE_WIDTH, STAGE_HEIGHT), area)
}

fn stage_rect(area: Rect) -> Rect {
    let [outer] = Layout::vertical([Constraint::Length(STAGE_HEIGHT)])
        .flex(Flex::Center)
        .areas(area);
    let [stage] = Layout::horizontal([Constraint::Length(STAGE_WIDTH)])
        .flex(Flex::Center)
        .areas(outer);
    stage
}

fn clamp_rect_to_area(rect: Rect, area: Rect) -> Rect {
    let max_x = area
        .x
        .saturating_add(area.width.saturating_sub(rect.width));
    let max_y = area
        .y
        .saturating_add(area.height.saturating_sub(rect.height));

    Rect::new(
        rect.x.clamp(area.x, max_x),
        rect.y.clamp(area.y, max_y),
        rect.width.min(area.width),
        rect.height.min(area.height),
    )
}

struct SignedRect {
    x: i16,
    y: i16,
    width: i16,
    height: i16,
}

impl SignedRect {
    fn contains(&self, x: i16, y: i16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    fn center_x(&self) -> i16 {
        self.x + self.width / 2
    }

    fn center_y(&self) -> i16 {
        self.y + self.height / 2
    }

    fn intersects_rect(&self, rect: Rect) -> bool {
        let other = SignedRect {
            x: rect.x as i16,
            y: rect.y as i16,
            width: rect.width as i16,
            height: rect.height as i16,
        };
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

fn mascot_lines(reaction_ticks: u8, enraged: bool) -> Vec<Line<'static>> {
    let wing_gap = wing_gap(reaction_ticks);
    let wing_pad = " ".repeat(wing_gap as usize);
    let orb_color = if reaction_ticks > 0 {
        Color::Indexed(230)
    } else if enraged {
        Color::Indexed(196)
    } else {
        Color::White
    };
    let body_color = if enraged { Color::Indexed(203) } else { Color::Indexed(229) };
    let leg_color = if enraged { Color::Indexed(196) } else { Color::LightRed };

    vec![
        line(vec![
            plain("        "),
            feather("▁▁▁▂▃▄▅▆▇██", Color::Indexed(221)),
            plain_owned(format!("{}{}", "      ", wing_pad)),
            feather("██▇▆▅▄▃▂▁▁▁", Color::Indexed(221)),
            plain("        "),
        ]),
        line(vec![
            plain("     "),
            feather("▂▃▄▅▆▇██████", Color::Indexed(220)),
            plain_owned(" ".repeat((6 + wing_gap) as usize)),
            orb("░▒▓█▓▒░", orb_color),
            plain_owned(" ".repeat((6 + wing_gap) as usize)),
            feather("██████▇▆▅▄▃▂", Color::Indexed(220)),
            plain("     "),
        ]),
        line(vec![
            plain("   "),
            feather("▃▄▅▆▇████████", Color::Indexed(220)),
            plain_owned(" ".repeat((4 + wing_gap) as usize)),
            orb("▒▓█████▓▒", orb_color),
            plain_owned(" ".repeat((4 + wing_gap) as usize)),
            feather("████████▇▆▅▄▃", Color::Indexed(220)),
            plain("   "),
        ]),
        line(vec![
            plain(" "),
            feather("▄▅▆▇██████████", Color::Indexed(214)),
            plain_owned(" ".repeat((3 + wing_gap) as usize)),
            orb("▓█████████▓", orb_color),
            plain_owned(" ".repeat((3 + wing_gap) as usize)),
            feather("██████████▇▆▅▄", Color::Indexed(214)),
            plain(" "),
        ]),
        line(vec![
            feather("▅▆▇███████", Color::Indexed(214)),
            plain_owned(" ".repeat((1 + wing_gap / 2) as usize)),
            feather("████", Color::Indexed(226)),
            plain_owned(" ".repeat((1 + wing_gap) as usize)),
            orb("███████████", orb_color),
            plain_owned(" ".repeat((1 + wing_gap) as usize)),
            feather("████", Color::Indexed(226)),
            plain_owned(" ".repeat((1 + wing_gap / 2) as usize)),
            feather("███████▇▆▅", Color::Indexed(214)),
        ]),
        line(vec![
            feather("▆▇██████", Color::Indexed(214)),
            plain_owned(" ".repeat((1 + wing_gap / 2) as usize)),
            feather("██████", Color::Indexed(226)),
            plain_owned(" ".repeat((1 + wing_gap) as usize)),
            orb("▓███████▓", orb_color),
            plain_owned(" ".repeat((1 + wing_gap) as usize)),
            feather("██████", Color::Indexed(226)),
            plain_owned(" ".repeat((1 + wing_gap / 2) as usize)),
            feather("██████▇▆", Color::Indexed(214)),
        ]),
        line(vec![
            plain("        "),
            feather("██████", Color::Indexed(226)),
            plain("   "),
            body("▄▄█████▄▄", body_color),
            plain("   "),
            feather("██████", Color::Indexed(226)),
            plain("        "),
        ]),
        line(vec![
            plain("         "),
            feather("████", Color::Indexed(226)),
            plain("    "),
            body("███▄███", body_color),
            plain("    "),
            feather("████", Color::Indexed(226)),
            plain("         "),
        ]),
        line(vec![
            plain("            "),
            body("██", body_color),
            plain("       "),
            body("██", body_color),
            plain("            "),
        ]),
        line(vec![
            plain("            "),
            leg("▐█", leg_color),
            plain("       "),
            leg("█▌", leg_color),
            plain("            "),
        ]),
        line(vec![
            plain("           "),
            leg("▐█", leg_color),
            plain("         "),
            leg("█▌", leg_color),
            plain("           "),
        ]),
        line(vec![
            plain("          "),
            leg("▐▀", leg_color),
            plain("           "),
            leg("▀▌", leg_color),
            plain("          "),
        ]),
    ]
}

fn ray_field(area: Rect, pulse: u32) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(area.height as usize);

    for y in 0..area.height as i32 {
        let mut spans = Vec::with_capacity(area.width as usize);

        for x in 0..area.width as i32 {
            let (dx, dy, distance) = ray_metrics(area, x, y);
            let glyph = ray_glyph(dx, dy, distance, pulse);
            let color = ray_color(distance, dx, dy, pulse);
            spans.push(Span::styled(glyph, Style::default().fg(color)));
        }

        lines.push(Line::from(spans));
    }

    lines
}
fn ray_metrics(area: Rect, x: i32, y: i32) -> (i32, i32, i32) {
    let center_x = area.width as i32 / 2;
    let center_y = area.height as i32 / 3;
    let dx = x - center_x;
    let dy = y - center_y;
    let distance = dx.abs() + dy.abs();
    (dx, dy, distance)
}
fn line(spans: Vec<Span<'static>>) -> Line<'static> {
    Line::from(spans)
}

fn plain(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::default().bg(Color::Reset))
}

fn plain_owned(text: String) -> Span<'static> {
    Span::styled(text, Style::default().bg(Color::Reset))
}

fn orb(text: &'static str, color: Color) -> Span<'static> {
    Span::styled(text, Style::default().fg(color).bg(Color::Reset))
}

fn leg(text: &'static str, color: Color) -> Span<'static> {
    Span::styled(text, Style::default().fg(color).bg(Color::Reset))
}

fn feather(text: &'static str, color: Color) -> Span<'static> {
    Span::styled(text, Style::default().fg(color).bg(Color::Reset))
}

fn body(text: &'static str, color: Color) -> Span<'static> {
    Span::styled(text, Style::default().fg(color).bg(Color::Reset))
}

fn ray_glyph(dx: i32, dy: i32, distance: i32, pulse: u32) -> &'static str {
    let glow = glow_phase(pulse);

    if distance < 3 {
        if glow >= 3 { "✹" } else { "✦" }
    } else if dx.abs() <= 1 {
        "│"
    } else if dy.abs() <= 1 {
        "─"
    } else if (dx.abs() - dy.abs()).abs() <= 1 {
        if (dx > 0) == (dy > 0) { "\\" } else { "/" }
    } else if (dx.abs() * 2 - dy.abs()).abs() <= 1 {
        if dy >= 0 { "╱" } else { "╲" }
    } else if (dy.abs() * 2 - dx.abs()).abs() <= 1 {
        if dx >= 0 { "╲" } else { "╱" }
    } else if (dx + dy + pulse_offset(pulse)).rem_euclid(17) == 0 {
        "·"
    } else {
        " "
    }
}

fn ray_color(distance: i32, dx: i32, dy: i32, pulse: u32) -> Color {
    let glow = glow_phase(pulse);

    if distance < 4 {
        match glow {
            0 | 1 => Color::White,
            2 => Color::Indexed(230),
            _ => Color::Indexed(229),
        }
    } else if distance < 8 {
        match glow {
            0 | 1 => Color::LightCyan,
            2 => Color::Indexed(159),
            _ => Color::Indexed(195),
        }
    } else if (dx.abs() - dy.abs()).abs() <= 1 || dx.abs() <= 1 || dy.abs() <= 1 {
        match glow {
            0 | 1 => Color::Indexed(223),
            2 => Color::Indexed(229),
            _ => Color::Indexed(221),
        }
    } else if (dx.abs() * 2 - dy.abs()).abs() <= 1 || (dy.abs() * 2 - dx.abs()).abs() <= 1 {
        match glow {
            0 | 1 => Color::Indexed(187),
            2 => Color::Indexed(195),
            _ => Color::Indexed(159),
        }
    } else {
        Color::Indexed(238)
    }
}

fn pulse_offset(pulse: u32) -> i32 {
    -(pulse as i32)
}

fn glow_phase(pulse: u32) -> u32 {
    pulse % 5
}

fn reaction_shift(reaction_ticks: u8) -> i16 {
    match reaction_ticks {
        7 | 4 | 1 => -1,
        6 | 3 => 1,
        _ => 0,
    }
}

fn wing_gap(reaction_ticks: u8) -> u16 {
    if reaction_ticks > 0 { 2 } else { 0 }
}
