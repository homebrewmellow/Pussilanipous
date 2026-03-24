use std::{
    io,
    io::stdout,
    time::{Duration, Instant},
};

mod emblem;
mod cursor;
mod menu;

use ratatui::{
    crossterm::{
        event::{
            self, Event, KeyCode, KeyEventKind, KeyboardEnhancementFlags,
            PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
        },
        execute,
        terminal::supports_keyboard_enhancement,
    },
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Line,
    Frame,
    widgets::{Block, Borders, Clear, Paragraph},
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let keyboard_enhancements = KeyboardEnhancements::enable_if_supported()?;
    let result = App::new(keyboard_enhancements.enabled).run(&mut terminal);
    drop(keyboard_enhancements);
    ratatui::restore();
    result
}

const MENU_WIDTH: u16 = 34;
const MENU_HEIGHT: u16 = 10;
const MENU_ANIMATION_STEPS: u16 = 6;
const RAY_DRIFT_STEPS: u32 = 17;
const FRAME_TIME: Duration = Duration::from_millis(8);
const MOVEMENT_FRAME_TIME: Duration = Duration::from_millis(16);
const EMBLEM_FRAME_TIME: Duration = Duration::from_millis(48);
const ANIMATION_FRAME_TIME: Duration = Duration::from_millis(120);
const INPUT_HOLD_WINDOW: Duration = Duration::from_millis(220);
const SPEED_BOOST_DELAY: Duration = Duration::from_millis(180);
const HORIZONTAL_STEP: i16 = 2;
const VERTICAL_STEP: i16 = 1;
const BOOST_MULTIPLIER: i16 = 2;
const ENRAGE_HIT_COUNT: u8 = 5;
const EMBLEM_CHASE_HORIZONTAL_STEP: i16 = 2;
const EMBLEM_CHASE_VERTICAL_STEP: i16 = 1;
const EMBLEM_GRACE_TICKS: u8 = 10;
const CAUGHT_SHAKE_TICKS: u8 = 8;
const EMBLEM_RESET_MIN_RADIUS: f32 = 1.5;
const EMBLEM_RESET_SPEED: f32 = 1.4;
const EMBLEM_RESET_TANGENTIAL_WEIGHT: f32 = 0.9;
const EMBLEM_RESET_RADIAL_WEIGHT: f32 = 0.6;
const TUTORIAL_BOX_WIDTH: u16 = 32;
const TUTORIAL_BOX_HEIGHT: u16 = 11;
const ESCAPE_TUTORIAL_DISTANCE: f32 = 18.0;

struct App {
    title: &'static str,
    tick: u32,
    menu: MenuState,
    input_mode: InputMode,
    tutorial: TutorialState,
    cursor: cursor::Cursor,
    emblem_state: EmblemState,
    caught_state: Option<CaughtState>,
    emblem_reaction: u8,
}

#[derive(Clone, Copy, Default)]
enum MenuState {
    #[default]
    Closed,
    Opening(u16),
    Open,
    Closing(u16),
}

#[derive(Clone, Copy, Default)]
struct DirectionalInput {
    up: DirectionState,
    down: DirectionState,
    left: DirectionState,
    right: DirectionState,
}

#[derive(Clone, Copy, Default)]
struct HeldDirections {
    up: DirectionState,
    down: DirectionState,
    left: DirectionState,
    right: DirectionState,
}

#[derive(Clone, Copy, Default)]
struct DirectionState {
    held_since: Option<Instant>,
    last_seen: Option<Instant>,
}

#[derive(Clone, Copy)]
enum InputMode {
    Enhanced(HeldDirections),
    Fallback(DirectionalInput),
}

struct KeyboardEnhancements {
    enabled: bool,
}

#[derive(Default)]
struct EmblemState {
    offset_x: i16,
    offset_y: i16,
    hit_count: u8,
    enraged: bool,
    grace_ticks: u8,
    reset_motion: Option<ResetMotion>,
}

#[derive(Clone, Copy)]
struct CaughtState {
    ticks_left: u8,
}

#[derive(Clone, Copy)]
struct ResetMotion {
    x: f32,
    y: f32,
}

#[derive(Default)]
struct TutorialState {
    stage: TutorialStage,
    movement: MovementTutorial,
    bird_hits: u8,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum TutorialStage {
    #[default]
    Movement,
    AnnoyBird,
    EscapeBird,
    Done,
}

#[derive(Default)]
struct MovementTutorial {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl App {
    fn new(enhanced_input: bool) -> Self {
        Self {
            title: "",
            tick: 0,
            menu: MenuState::Closed,
            input_mode: InputMode::new(enhanced_input),
            tutorial: TutorialState::default(),
            cursor: cursor::Cursor::default(),
            emblem_state: EmblemState::default(),
            caught_state: None,
            emblem_reaction: 0,
        }
    }

    fn run(mut self, terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
        if self.title.is_empty() {
            self.title = "Pusillanipous";
        }

        let mut dirty = true;
        let mut last_movement = Instant::now();
        let mut last_emblem_motion = Instant::now();
        let mut last_animation = Instant::now();

        loop {
            while event::poll(Duration::ZERO)? {
                if let Event::Key(key) = event::read()? {
                    let event_time = Instant::now();
                    match key.kind {
                        KeyEventKind::Press => match key.code {
                            KeyCode::Char('e') | KeyCode::Char('E') => {
                                self.open_menu();
                                dirty = true;
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                let size = terminal.size()?;
                                let area = Rect::new(0, 0, size.width, size.height);
                                let (x, y) = emblem::center(
                                    area,
                                    self.emblem_state.offset_x,
                                    self.emblem_state.offset_y,
                                );
                                dirty |= self.cursor.recenter_on(area, x, y);
                            }
                            KeyCode::Char('q') | KeyCode::Esc => match self.menu {
                                MenuState::Closed => return Ok(()),
                                _ => {
                                    self.close_menu();
                                    dirty = true;
                                }
                            },
                            KeyCode::Up => {
                                self.tutorial.mark_up();
                                self.input_mode.press_up(event_time);
                                dirty = true;
                            }
                            KeyCode::Down => {
                                self.tutorial.mark_down();
                                self.input_mode.press_down(event_time);
                                dirty = true;
                            }
                            KeyCode::Left => {
                                self.tutorial.mark_left();
                                self.input_mode.press_left(event_time);
                                dirty = true;
                            }
                            KeyCode::Right => {
                                self.tutorial.mark_right();
                                self.input_mode.press_right(event_time);
                                dirty = true;
                            }
                            KeyCode::Enter => {
                                let size = terminal.size()?;
                                let area = Rect::new(0, 0, size.width, size.height);
                                let (x, y) = self.cursor.center(area);
                                if emblem::is_cursor_over(
                                    area,
                                    x,
                                    y,
                                    self.emblem_state.offset_x,
                                    self.emblem_state.offset_y,
                                ) {
                                    self.emblem_state.register_hit();
                                    self.tutorial.record_emblem_hit(self.emblem_state.hit_count);
                                    self.emblem_reaction = 8;
                                    dirty = true;
                                }
                            }
                            _ => {}
                        },
                        KeyEventKind::Repeat => match key.code {
                            KeyCode::Up => self.input_mode.press_up(event_time),
                            KeyCode::Down => self.input_mode.press_down(event_time),
                            KeyCode::Left => self.input_mode.press_left(event_time),
                            KeyCode::Right => self.input_mode.press_right(event_time),
                            _ => {}
                        },
                        KeyEventKind::Release => match key.code {
                            KeyCode::Up => self.input_mode.release_up(),
                            KeyCode::Down => self.input_mode.release_down(),
                            KeyCode::Left => self.input_mode.release_left(),
                            KeyCode::Right => self.input_mode.release_right(),
                            _ => {}
                        },
                    }
                }
            }

            let now = Instant::now();
            if now.duration_since(last_movement) >= MOVEMENT_FRAME_TIME {
                if self.cursor.is_recentering() && self.caught_state.is_none() {
                    let size = terminal.size()?;
                    let area = Rect::new(0, 0, size.width, size.height);
                    let (x, y) = emblem::center(
                        area,
                        self.emblem_state.offset_x,
                        self.emblem_state.offset_y,
                    );
                    self.cursor.retarget_recenter(area, x, y);
                }
                if self.caught_state.is_none() {
                    dirty |= self.advance_cursor(now);
                } else {
                    dirty = true;
                }
                last_movement = now;
            }

            if now.duration_since(last_emblem_motion) >= EMBLEM_FRAME_TIME {
                let size = terminal.size()?;
                let area = Rect::new(0, 0, size.width, size.height);
                let (cursor_x, cursor_y) = self.cursor.center(area);
                let cursor_rect = self.cursor.rect(area);

                if let Some(caught) = self.caught_state.as_mut() {
                    caught.ticks_left = caught.ticks_left.saturating_sub(1);
                    if caught.ticks_left == 0 {
                        self.caught_state = None;
                        self.emblem_state.reset();
                        self.emblem_reaction = 0;
                    }
                    dirty = true;
                } else {
                    dirty |= self.emblem_state.tick(area, cursor_x, cursor_y);
                    let (emblem_x, emblem_y) =
                        emblem::stage_center(area, self.emblem_state.offset_x, self.emblem_state.offset_y);
                    let distance = offset_radius_f32(
                        cursor_x as f32 - emblem_x as f32,
                        cursor_y as f32 - emblem_y as f32,
                    );
                    self.tutorial.observe_emblem(self.emblem_state.enraged, distance);
                    if self.emblem_state.enraged
                        && !self.emblem_state.in_grace_period()
                        && emblem::overlaps_cursor(
                            area,
                            cursor_rect,
                            self.emblem_state.offset_x,
                            self.emblem_state.offset_y,
                        )
                    {
                        self.caught_state = Some(CaughtState {
                            ticks_left: CAUGHT_SHAKE_TICKS,
                        });
                        self.cursor.cancel_recenter();
                        dirty = true;
                    }
                }

                last_emblem_motion = now;
            }

            if now.duration_since(last_animation) >= ANIMATION_FRAME_TIME {
                self.tick = self.tick.wrapping_add(1);
                self.advance_menu_animation();
                self.emblem_reaction = self.emblem_reaction.saturating_sub(1);
                last_animation = now;
                dirty = true;
            }

            if dirty {
                terminal.draw(|frame| self.draw(frame))?;
                dirty = false;
            }

            let _ = event::poll(FRAME_TIME)?;
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        emblem::render(
            frame,
            area,
            self.pulse_phase(),
            self.emblem_reaction,
            self.emblem_state.offset_x,
            self.emblem_state.offset_y,
            self.emblem_state.enraged,
        );

        let [top, _, bottom] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
                .areas(area);

        frame.render_widget(
            Paragraph::new(self.title)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Indexed(230))),
            top,
        );
        frame.render_widget(
            Paragraph::new(if self.menu_visible() {
                "Arrows move cursor • c recenters • q closes menu"
            } else if self.caught_state.is_some() {
                "The emblem caught you, shook you, and will reset"
            } else if self.emblem_state.in_grace_period() {
                "The emblem is waking up. Move."
            } else if self.emblem_state.is_resetting() {
                "The emblem is spiraling back into place"
            } else if self.emblem_state.enraged {
                "The emblem is enraged and stalking you"
            } else if self.emblem_reaction > 0 {
                "Enter on emblem awakened it"
            } else {
                "E opens menu • C recenters • Enter touches emblem • q exits"
            })
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Gray)),
            bottom,
        );

        if self.menu_visible() {
            self.draw_menu(frame);
        }

        if !self.tutorial.complete() {
            self.draw_tutorial(frame, area);
        }

        let (shake_x, shake_y) = self.cursor_shake_offset();
        self.cursor.render_with_offset(frame, area, shake_x, shake_y);
    }

    fn draw_tutorial(&mut self, frame: &mut Frame, area: Rect) {
        let cursor_area = self.cursor.rect(area);
        let tutorial_area = tutorial_box_rect(area, cursor_area, self.menu_visible());
        let lines = self.tutorial.render_lines();
        let block = Block::default()
            .title(self.tutorial.title())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Indexed(223)));
        let inner = block.inner(tutorial_area);

        frame.render_widget(Clear, tutorial_area);
        frame.render_widget(block, tutorial_area);

        let buf = frame.buffer_mut();
        for (index, line) in lines.iter().enumerate() {
            let y = inner.y.saturating_add(index as u16);
            if y >= inner.y.saturating_add(inner.height) {
                break;
            }

            let line_width = line.width() as u16;
            let x = inner.x + inner.width.saturating_sub(line_width) / 2;
            buf.set_line(x, y, line, inner.width);
        }
    }

    fn advance_cursor(&mut self, now: Instant) -> bool {
        let (dx, dy) = self.input_mode.axis(now);

        if dx != 0 || dy != 0 {
            self.cursor.move_by(dx, dy)
        } else {
            self.cursor.tick()
        }
    }

    fn pulse_phase(&self) -> u32 {
        self.tick % RAY_DRIFT_STEPS
    }

    fn cursor_shake_offset(&self) -> (i16, i16) {
        match self.caught_state {
            Some(CaughtState { ticks_left }) => match ticks_left % 6 {
                0 => (1, 0),
                1 => (-1, 0),
                2 => (0, 1),
                3 => (0, -1),
                4 => (1, 1),
                _ => (-1, -1),
            },
            None => (0, 0),
        }
    }

    fn menu_visible(&self) -> bool {
        !matches!(self.menu, MenuState::Closed)
    }

    fn open_menu(&mut self) {
        self.menu = match self.menu {
            MenuState::Closed => MenuState::Opening(0),
            MenuState::Closing(step) => MenuState::Opening(MENU_ANIMATION_STEPS - step),
            state => state,
        };
    }

    fn close_menu(&mut self) {
        self.menu = match self.menu {
            MenuState::Open => MenuState::Closing(MENU_ANIMATION_STEPS),
            MenuState::Opening(step) => MenuState::Closing(step),
            state => state,
        };
    }

    fn advance_menu_animation(&mut self) {
        self.menu = match self.menu {
            MenuState::Opening(step) if step >= MENU_ANIMATION_STEPS => MenuState::Open,
            MenuState::Opening(step) => MenuState::Opening(step + 1),
            MenuState::Closing(step) if step <= 1 => MenuState::Closed,
            MenuState::Closing(step) => MenuState::Closing(step - 1),
            state => state,
        };
    }

    fn menu_progress(&self) -> u16 {
        match self.menu {
            MenuState::Closed => 0,
            MenuState::Opening(step) => step,
            MenuState::Open => MENU_ANIMATION_STEPS,
            MenuState::Closing(step) => step,
        }
    }

    fn draw_menu(&self, frame: &mut Frame) {
        let area = frame.area();
        let progress = self.menu_progress();
        let menu_area = centered_rect(
            area,
            scaled_dimension(MENU_WIDTH, progress),
            scaled_dimension(MENU_HEIGHT, progress),
        );

        if menu_area.width < 2 || menu_area.height < 2 {
            return;
        }
        let menu_lines = if progress == MENU_ANIMATION_STEPS {
            menu::render_lines()
        } else {
            Vec::new()
        };

        frame.render_widget(Clear, menu_area);
        frame.render_widget(
            Block::default()
                .title(if progress == MENU_ANIMATION_STEPS { "About" } else { "" })
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Indexed(229))),
            menu_area,
        );

        if progress == MENU_ANIMATION_STEPS {
            let inner = Block::default().borders(Borders::ALL).inner(menu_area);
            let buf = frame.buffer_mut();
            for (index, line) in menu_lines.iter().enumerate() {
                let y = inner.y.saturating_add(index as u16);
                if y >= inner.y.saturating_add(inner.height) {
                    break;
                }
                buf.set_line(inner.x, y, line, inner.width);
            }
        }
    }
}

impl InputMode {
    fn new(enhanced_input: bool) -> Self {
        if enhanced_input {
            Self::Enhanced(HeldDirections::default())
        } else {
            Self::Fallback(DirectionalInput::default())
        }
    }

    fn press_up(&mut self, now: Instant) {
        match self {
            Self::Enhanced(held) => held.up.press(now),
            Self::Fallback(input) => input.up.press(now),
        }
    }

    fn press_down(&mut self, now: Instant) {
        match self {
            Self::Enhanced(held) => held.down.press(now),
            Self::Fallback(input) => input.down.press(now),
        }
    }

    fn press_left(&mut self, now: Instant) {
        match self {
            Self::Enhanced(held) => held.left.press(now),
            Self::Fallback(input) => input.left.press(now),
        }
    }

    fn press_right(&mut self, now: Instant) {
        match self {
            Self::Enhanced(held) => held.right.press(now),
            Self::Fallback(input) => input.right.press(now),
        }
    }

    fn release_up(&mut self) {
        match self {
            Self::Enhanced(held) => held.up.release(),
            Self::Fallback(input) => input.up.release(),
        }
    }

    fn release_down(&mut self) {
        match self {
            Self::Enhanced(held) => held.down.release(),
            Self::Fallback(input) => input.down.release(),
        }
    }

    fn release_left(&mut self) {
        match self {
            Self::Enhanced(held) => held.left.release(),
            Self::Fallback(input) => input.left.release(),
        }
    }

    fn release_right(&mut self) {
        match self {
            Self::Enhanced(held) => held.right.release(),
            Self::Fallback(input) => input.right.release(),
        }
    }

    fn axis(&self, now: Instant) -> (i16, i16) {
        match self {
            Self::Enhanced(held) => (
                horizontal_velocity(held.left, held.right, now, true),
                vertical_velocity(held.up, held.down, now, true),
            ),
            Self::Fallback(input) => (
                horizontal_velocity(input.left, input.right, now, false),
                vertical_velocity(input.up, input.down, now, false),
            ),
        }
    }
}

impl KeyboardEnhancements {
    fn enable_if_supported() -> io::Result<Self> {
        if !matches!(supports_keyboard_enhancement(), Ok(true)) {
            return Ok(Self { enabled: false });
        }

        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        )?;

        Ok(Self { enabled: true })
    }
}

impl Drop for KeyboardEnhancements {
    fn drop(&mut self) {
        if self.enabled {
            let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
        }
    }
}

impl EmblemState {
    fn register_hit(&mut self) {
        self.hit_count = self.hit_count.saturating_add(1);
        if self.hit_count >= ENRAGE_HIT_COUNT && !self.enraged {
            self.enraged = true;
            self.grace_ticks = EMBLEM_GRACE_TICKS;
        }
    }

    fn tick(&mut self, area: Rect, cursor_x: u16, cursor_y: u16) -> bool {
        if self.tick_reset() {
            return true;
        }

        if !self.enraged {
            return false;
        }

        if self.grace_ticks > 0 {
            self.grace_ticks = self.grace_ticks.saturating_sub(1);
            return true;
        }

        let (emblem_x, emblem_y) = emblem::stage_center(area, self.offset_x, self.offset_y);
        let desired_x = cursor_x as i16 - emblem_x;
        let desired_y = cursor_y as i16 - emblem_y;
        let step_x = clamp_step(desired_x, EMBLEM_CHASE_HORIZONTAL_STEP);
        let step_y = clamp_step(desired_y, EMBLEM_CHASE_VERTICAL_STEP);

        if step_x == 0 && step_y == 0 {
            return false;
        }

        self.offset_x += step_x;
        self.offset_y += step_y;
        true
    }

    fn reset(&mut self) {
        self.hit_count = 0;
        self.enraged = false;
        self.grace_ticks = 0;
        self.reset_motion = Some(ResetMotion {
            x: self.offset_x as f32,
            y: self.offset_y as f32,
        });
        if self.offset_x == 0 && self.offset_y == 0 {
            self.reset_motion = None;
        }
    }

    fn is_resetting(&self) -> bool {
        self.reset_motion.is_some()
    }

    fn in_grace_period(&self) -> bool {
        self.enraged && self.grace_ticks > 0
    }

    fn tick_reset(&mut self) -> bool {
        let Some(motion) = self.reset_motion.as_mut() else {
            return false;
        };

        let distance = offset_radius_f32(motion.x, motion.y);
        if distance <= f32::EPSILON {
            self.offset_x = 0;
            self.offset_y = 0;
            self.reset_motion = None;
            return false;
        }

        if distance <= EMBLEM_RESET_MIN_RADIUS {
            motion.x = 0.0;
            motion.y = 0.0;
        } else {
            let radial_x = -motion.x / distance;
            let radial_y = -motion.y / distance;
            let tangent_x = -motion.y / distance;
            let tangent_y = motion.x / distance;

            let velocity_x = tangent_x * EMBLEM_RESET_TANGENTIAL_WEIGHT
                + radial_x * EMBLEM_RESET_RADIAL_WEIGHT;
            let velocity_y = tangent_y * EMBLEM_RESET_TANGENTIAL_WEIGHT
                + radial_y * EMBLEM_RESET_RADIAL_WEIGHT;
            let velocity_length = offset_radius_f32(velocity_x, velocity_y).max(f32::EPSILON);

            motion.x += velocity_x / velocity_length * EMBLEM_RESET_SPEED;
            motion.y += velocity_y / velocity_length * EMBLEM_RESET_SPEED;
        }

        self.offset_x = motion.x.round() as i16;
        self.offset_y = motion.y.round() as i16;

        if self.offset_x == 0 && self.offset_y == 0 {
            self.reset_motion = None;
        }

        true
    }
}

impl DirectionState {
    fn press(&mut self, now: Instant) {
        if self.held_since.is_none() {
            self.held_since = Some(now);
        }
        self.last_seen = Some(now);
    }

    fn release(&mut self) {
        self.held_since = None;
        self.last_seen = None;
    }

    fn is_active(&self, now: Instant, enhanced: bool) -> bool {
        if enhanced {
            self.held_since.is_some()
        } else {
            self.last_seen
                .is_some_and(|instant| now.duration_since(instant) <= INPUT_HOLD_WINDOW)
        }
    }

    fn boosted(&self, now: Instant, enhanced: bool) -> bool {
        self.is_active(now, enhanced)
            && self
                .held_since
                .is_some_and(|instant| now.duration_since(instant) >= SPEED_BOOST_DELAY)
    }
}

fn horizontal_velocity(left: DirectionState, right: DirectionState, now: Instant, enhanced: bool) -> i16 {
    directional_velocity(left, right, now, enhanced, HORIZONTAL_STEP)
}

fn vertical_velocity(up: DirectionState, down: DirectionState, now: Instant, enhanced: bool) -> i16 {
    directional_velocity(up, down, now, enhanced, VERTICAL_STEP)
}

fn directional_velocity(
    negative: DirectionState,
    positive: DirectionState,
    now: Instant,
    enhanced: bool,
    base_step: i16,
) -> i16 {
    let negative_active = negative.is_active(now, enhanced);
    let positive_active = positive.is_active(now, enhanced);

    if negative_active == positive_active {
        return 0;
    }

    let state = if positive_active { positive } else { negative };
    let step = if state.boosted(now, enhanced) {
        base_step * BOOST_MULTIPLIER
    } else {
        base_step
    };

    if positive_active { step } else { -step }
}

impl TutorialState {
    fn mark_up(&mut self) {
        if self.stage == TutorialStage::Movement {
            self.movement.mark_up();
            self.advance_if_ready();
        }
    }

    fn mark_down(&mut self) {
        if self.stage == TutorialStage::Movement {
            self.movement.mark_down();
            self.advance_if_ready();
        }
    }

    fn mark_left(&mut self) {
        if self.stage == TutorialStage::Movement {
            self.movement.mark_left();
            self.advance_if_ready();
        }
    }

    fn mark_right(&mut self) {
        if self.stage == TutorialStage::Movement {
            self.movement.mark_right();
            self.advance_if_ready();
        }
    }

    fn record_emblem_hit(&mut self, hit_count: u8) {
        if self.stage == TutorialStage::AnnoyBird {
            self.bird_hits = hit_count.min(ENRAGE_HIT_COUNT);
        }
    }

    fn observe_emblem(&mut self, enraged: bool, distance: f32) {
        if self.stage == TutorialStage::AnnoyBird && enraged {
            self.stage = TutorialStage::EscapeBird;
        }

        if self.stage == TutorialStage::EscapeBird && distance >= ESCAPE_TUTORIAL_DISTANCE {
            self.stage = TutorialStage::Done;
        }
    }

    fn complete(&self) -> bool {
        self.stage == TutorialStage::Done
    }

    fn title(&self) -> &'static str {
        match self.stage {
            TutorialStage::Movement => "Movement",
            TutorialStage::AnnoyBird => "Bird",
            TutorialStage::EscapeBird => "Escape",
            TutorialStage::Done => "",
        }
    }

    fn render_lines(&self) -> Vec<Line<'static>> {
        match self.stage {
            TutorialStage::Movement => self.movement.render_lines(),
            TutorialStage::AnnoyBird => vec![
                Line::styled("Try to annoy the bird", Style::default().fg(Color::Indexed(229))),
                Line::raw(""),
                Line::from("      [ Enter ]"),
                Line::raw(""),
                Line::styled("Press Enter on it a bunch", Style::default().fg(Color::Indexed(187))),
                Line::styled(
                    format!("Progress {}/{}", self.bird_hits, ENRAGE_HIT_COUNT),
                    Style::default().fg(Color::Indexed(159)),
                ),
            ],
            TutorialStage::EscapeBird => vec![
                Line::styled("The bird is angry now", Style::default().fg(Color::Indexed(229))),
                Line::raw(""),
                Line::styled("Gain some distance", Style::default().fg(Color::Indexed(187))),
                Line::styled("it will hunt you", Style::default().fg(Color::Indexed(187))),
                Line::styled("until it gets back at you", Style::default().fg(Color::Indexed(187))),
                Line::raw(""),
                Line::styled("Keep flying", Style::default().fg(Color::Indexed(159))),
            ],
            TutorialStage::Done => Vec::new(),
        }
    }

    fn advance_if_ready(&mut self) {
        if self.stage == TutorialStage::Movement && self.movement.complete() {
            self.stage = TutorialStage::AnnoyBird;
        }
    }
}

impl MovementTutorial {
    fn mark_up(&mut self) {
        self.up = true;
    }

    fn mark_down(&mut self) {
        self.down = true;
    }

    fn mark_left(&mut self) {
        self.left = true;
    }

    fn mark_right(&mut self) {
        self.right = true;
    }

    fn complete(&self) -> bool {
        self.up && self.down && self.left && self.right
    }

    fn render_lines(&self) -> Vec<Line<'static>> {
        vec![
            Line::styled("Try every arrow key", Style::default().fg(Color::Indexed(229))),
            Line::raw(""),
            Line::from(format!("    {}    ", self.keycap("↑", self.up))),
            Line::from(
                self.keycap("←", self.left) + " " + &self.keycap("↓", self.down) + " " + &self.keycap("→", self.right),
            ),
            Line::raw(""),
            Line::styled(
                format!("Progress {}/4", self.count_complete()),
                Style::default().fg(Color::Indexed(187)),
            ),
        ]
    }

    fn count_complete(&self) -> usize {
        self.up as usize + self.down as usize + self.left as usize + self.right as usize
    }

    fn keycap(&self, label: &'static str, done: bool) -> String {
        if done {
            format!("[{}]", label)
        } else {
            format!("({})", label)
        }
    }
}

fn clamp_step(delta: i16, limit: i16) -> i16 {
    delta.clamp(-limit, limit)
}

fn offset_radius_f32(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

fn tutorial_box_rect(area: Rect, cursor: Rect, menu_visible: bool) -> Rect {
    let preferred_right_x = cursor.x.saturating_add(5);
    let max_x = area.x + area.width.saturating_sub(TUTORIAL_BOX_WIDTH);
    let min_x = area.x;
    let right_x = preferred_right_x.clamp(min_x, max_x);

    let left_x = cursor
        .x
        .saturating_sub(TUTORIAL_BOX_WIDTH.saturating_add(2))
        .clamp(min_x, max_x);

    let mut x = if right_x + TUTORIAL_BOX_WIDTH <= area.x + area.width {
        right_x
    } else {
        left_x
    };

    if menu_visible {
        x = left_x;
    }

    let preferred_y = cursor.y.saturating_sub(2);
    let max_y = area.y + area.height.saturating_sub(TUTORIAL_BOX_HEIGHT + 1);
    let y = preferred_y.clamp(area.y + 1, max_y.max(area.y + 1));

    Rect::new(x, y, TUTORIAL_BOX_WIDTH, TUTORIAL_BOX_HEIGHT)
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let [middle] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [center] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(middle);
    center
}

fn scaled_dimension(full: u16, progress: u16) -> u16 {
    let size = full.saturating_mul(progress) / MENU_ANIMATION_STEPS;
    size.max(1)
}
