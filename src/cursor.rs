use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    Frame,
    widgets::Paragraph,
};

const GHOST_LENGTH: usize = 24;
const RECENTER_STEPS_PER_TICK: usize = 2;
const VORTEX_ANGLE_STEP: f32 = 0.7;
const VORTEX_RADIUS_DECAY: f32 = 0.93;
const VORTEX_MIN_RADIUS: f32 = 1.5;

#[derive(Default)]
pub struct Cursor {
    x: u16,
    y: u16,
    initialized: bool,
    target: Option<(u16, u16)>,
    vortex: Option<VortexMotion>,
    trail: Vec<(u16, u16)>,
}

#[derive(Clone, Copy)]
struct VortexMotion {
    angle: f32,
    radius: f32,
}

impl Cursor {
    pub fn move_by(&mut self, dx: i16, dy: i16) -> bool {
        self.target = None;
        self.vortex = None;
        let old_x = self.x;
        let old_y = self.y;
        let new_x = apply_delta(self.x, dx);
        let new_y = apply_delta(self.y, dy);
        let moved = new_x != old_x || new_y != old_y;

        if moved {
            self.record_path_to(new_x, new_y);
        }

        self.x = new_x;
        self.y = new_y;
        moved
    }

    pub fn render_with_offset(&mut self, frame: &mut Frame, area: Rect, offset_x: i16, offset_y: i16) {
        self.ensure_in_bounds(area);

        for (index, &(x, y)) in self.trail.iter().rev().enumerate() {
            let ghost_area = Rect::new(x, y, 3, 3);
            let glyph = ghost_glyph(index);
            frame.render_widget(Paragraph::new(glyph), ghost_area);
        }

        let cursor_area = Rect::new(
            self.x.saturating_add_signed(offset_x).clamp(area.x, area.x + area.width.saturating_sub(3)),
            self.y.saturating_add_signed(offset_y).clamp(area.y, area.y + area.height.saturating_sub(3)),
            3,
            3,
        );
        let glyph = vec![
            Line::styled("▛▀▜", Style::default().fg(Color::Indexed(230))),
            Line::styled("▌█▐", Style::default().fg(Color::Indexed(159))),
            Line::styled("▙▄▟", Style::default().fg(Color::Indexed(230))),
        ];

        frame.render_widget(Paragraph::new(glyph), cursor_area);
    }

    pub fn center(&mut self, area: Rect) -> (u16, u16) {
        self.ensure_in_bounds(area);
        (self.x + 1, self.y + 1)
    }

    pub fn rect(&mut self, area: Rect) -> Rect {
        self.ensure_in_bounds(area);
        Rect::new(self.x, self.y, 3, 3)
    }

    pub fn recenter_on(&mut self, area: Rect, x: u16, y: u16) -> bool {
        self.ensure_in_bounds(area);
        let target = clamp_target(area, x, y);
        let offset_x = self.x as f32 - target.0 as f32;
        let offset_y = self.y as f32 - target.1 as f32;
        let changed = self.target != Some(target);
        self.target = Some(target);
        self.vortex = Some(VortexMotion {
            angle: offset_y.atan2(offset_x),
            radius: (offset_x * offset_x + offset_y * offset_y).sqrt(),
        });
        changed
    }

    pub fn retarget_recenter(&mut self, area: Rect, x: u16, y: u16) -> bool {
        self.ensure_in_bounds(area);
        if self.target.is_none() {
            return false;
        }

        let target = clamp_target(area, x, y);
        let changed = self.target != Some(target);
        self.target = Some(target);
        changed
    }

    pub fn is_recentering(&self) -> bool {
        self.target.is_some()
    }

    pub fn cancel_recenter(&mut self) {
        self.target = None;
        self.vortex = None;
    }

    fn ensure_in_bounds(&mut self, area: Rect) {
        let max_x = area.width.saturating_sub(3);
        let max_y = area.height.saturating_sub(3);

        if !self.initialized {
            self.x = area.x + max_x / 2;
            self.y = area.y + max_y / 2;
            self.initialized = true;
            return;
        }

        self.x = self.x.clamp(area.x, area.x + max_x);
        self.y = self.y.clamp(area.y, area.y + max_y);
        self.target = self
            .target
            .map(|(x, y)| (x.clamp(area.x, area.x + max_x), y.clamp(area.y, area.y + max_y)));
        self.vortex = self.target.map(|(target_x, target_y)| {
            let offset_x = self.x as f32 - target_x as f32;
            let offset_y = self.y as f32 - target_y as f32;
            VortexMotion {
                angle: offset_y.atan2(offset_x),
                radius: (offset_x * offset_x + offset_y * offset_y).sqrt(),
            }
        });
        self.trail
            .retain(|&(x, y)| x >= area.x && y >= area.y && x <= area.x + max_x && y <= area.y + max_y);
    }

    fn record_path_to(&mut self, target_x: u16, target_y: u16) {
        let mut x = self.x;
        let mut y = self.y;

        while x != target_x || y != target_y {
            self.trail.push((x, y));

            if x != target_x {
                x = step_toward_by(x, target_x, 1);
            }

            if y != target_y {
                y = step_toward_by(y, target_y, 1);
            }
        }

        while self.trail.len() > GHOST_LENGTH {
            self.trail.remove(0);
        }
    }

    pub fn tick(&mut self) -> bool {
        let mut dirty = false;
        let mut moved = false;

        if let Some((target_x, target_y)) = self.target {
            if self.x != target_x || self.y != target_y {
                let vortex = self.vortex.get_or_insert_with(|| {
                    let offset_x = self.x as f32 - target_x as f32;
                    let offset_y = self.y as f32 - target_y as f32;
                    VortexMotion {
                        angle: offset_y.atan2(offset_x),
                        radius: (offset_x * offset_x + offset_y * offset_y).sqrt(),
                    }
                });
                let (new_x, new_y) = vortex_step(
                    self.x,
                    self.y,
                    target_x,
                    target_y,
                    vortex,
                    RECENTER_STEPS_PER_TICK,
                );
                self.record_path_to(new_x, new_y);
                self.x = new_x;
                self.y = new_y;
                dirty = true;
                moved = true;
            }

            if self.x == target_x && self.y == target_y {
                self.target = None;
                self.vortex = None;
            }
        }

        if moved || self.trail.is_empty() {
            return dirty;
        }

        self.trail.remove(0);
        true
    }
}

fn step_toward_by(value: u16, target: u16, amount: u16) -> u16 {
    match value.cmp(&target) {
        std::cmp::Ordering::Less => value.saturating_add(amount).min(target),
        std::cmp::Ordering::Greater => value.saturating_sub(amount).max(target),
        std::cmp::Ordering::Equal => value,
    }
}

fn clamp_target(area: Rect, x: u16, y: u16) -> (u16, u16) {
    let max_x = area.x + area.width.saturating_sub(3);
    let max_y = area.y + area.height.saturating_sub(3);
    (
        x.saturating_sub(1).clamp(area.x, max_x),
        y.saturating_sub(1).clamp(area.y, max_y),
    )
}

fn vortex_step(
    start_x: u16,
    start_y: u16,
    target_x: u16,
    target_y: u16,
    motion: &mut VortexMotion,
    steps: usize,
) -> (u16, u16) {
    let mut x = start_x;
    let mut y = start_y;

    for _ in 0..steps {
        let dx = target_x as i16 - x as i16;
        let dy = target_y as i16 - y as i16;

        if dx == 0 && dy == 0 {
            break;
        }

        let distance = ((dx * dx + dy * dy) as f32).sqrt();
        let (next_x, next_y) = if distance <= VORTEX_MIN_RADIUS {
            (
                step_toward_by(x, target_x, 1),
                step_toward_by(y, target_y, 1),
            )
        } else {
            motion.angle += VORTEX_ANGLE_STEP;
            motion.radius = (motion.radius * VORTEX_RADIUS_DECAY).max(VORTEX_MIN_RADIUS);

            let orbit_x = target_x as f32 + motion.radius * motion.angle.cos();
            let orbit_y = target_y as f32 + motion.radius * motion.angle.sin();
            let desired_x = orbit_x.round().max(0.0) as u16;
            let desired_y = orbit_y.round().max(0.0) as u16;

            (
                step_toward_by(x, desired_x, 1),
                step_toward_by(y, desired_y, 1),
            )
        };

        x = next_x;
        y = next_y;
    }

    (x, y)
}

fn apply_delta(value: u16, delta: i16) -> u16 {
    if delta >= 0 {
        value.saturating_add(delta as u16)
    } else {
        value.saturating_sub(delta.unsigned_abs())
    }
}

fn ghost_glyph(index: usize) -> Vec<Line<'static>> {
    let (edge, core) = match index {
        0 => (Color::Indexed(188), Color::Indexed(117)),
        1 => (Color::Indexed(145), Color::Indexed(74)),
        2 => (Color::Indexed(103), Color::Indexed(67)),
        _ => (Color::Indexed(60), Color::Indexed(60)),
    };

    vec![
        Line::styled("▛▀▜", Style::default().fg(edge)),
        Line::styled("▌█▐", Style::default().fg(core)),
        Line::styled("▙▄▟", Style::default().fg(edge)),
    ]
}
