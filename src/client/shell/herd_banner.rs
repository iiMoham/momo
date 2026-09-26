//! Animated pixel herd banner at the top of the expanded sidebar.
//!
//! One half-block pixel sheep per agent: wool color shows the agent's state,
//! the herd walks while agents work (faster with more working agents), a sheep
//! whose agent needs you gets a blinking `!`, and sheep sleep when nothing
//! works. Frames are a pure function of elapsed time, so motion speed does not
//! depend on the frame rate and every frame is unit-testable.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::api::schema::AgentStatus;
use crate::app::state::Palette;

/// Rows the banner takes: three pixel-art rows and one status line.
pub(super) const BANNER_ROWS: u16 = 4;
/// Shorter sidebars keep all their rows for Spaces and Agents.
const MIN_SIDEBAR_ROWS: u16 = 16;
const MIN_SIDEBAR_COLS: u16 = 12;
const PIXEL_ROWS: usize = 6;
const SHEEP_WIDTH: usize = 7;
const SHEEP_SPAN: usize = 9;
const SLOWEST_COLUMNS_PER_SECOND: f64 = 3.0;
const FASTEST_COLUMNS_PER_SECOND: f64 = 8.0;
const ALERT_BLINK: Duration = Duration::from_millis(500);
const SNORE_BLINK: Duration = Duration::from_millis(800);
const GROUND_SPACING: usize = 7;

// Sprites face right. W = wool, H = head, L = legs; rows are pixel rows.
const BODY: [&str; 3] = [".WWW...", "WWWWWHH", "WWWWWH."];
const LEGS_APART: [&str; 2] = [".L..L..", ".L..L.."];
const LEGS_TOGETHER: [&str; 2] = ["..LL...", "..LL..."];
const SLEEPING: [&str; 3] = [".WWW...", "WWWWWH.", "WWWWWHH"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HerdCell {
    pub(super) ch: char,
    pub(super) fg: Option<Color>,
    pub(super) bg: Option<Color>,
    pub(super) bold: bool,
}

const EMPTY: HerdCell = HerdCell {
    ch: ' ',
    fg: None,
    bg: None,
    bold: false,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pose {
    Walking { legs_apart: bool },
    Standing,
    Sleeping,
}

pub(super) fn fits(sidebar: Rect) -> bool {
    sidebar.height >= MIN_SIDEBAR_ROWS && sidebar.width >= MIN_SIDEBAR_COLS
}

/// Needs-you first, then working, done, idle, so the sheep that matter most
/// stay visible when the sidebar is too narrow for the whole herd.
fn herd_order(statuses: &[AgentStatus]) -> Vec<AgentStatus> {
    let rank = |status: &AgentStatus| match status {
        AgentStatus::Blocked => 0,
        AgentStatus::Working => 1,
        AgentStatus::Done => 2,
        AgentStatus::Idle => 3,
        AgentStatus::Unknown => 4,
    };
    let mut herd = statuses.to_vec();
    herd.sort_by_key(rank);
    herd
}

fn wool_color(status: AgentStatus, palette: &Palette) -> Color {
    super::status_color(status, palette)
}

/// Columns the herd has walked after `elapsed`, or `None` while nothing works.
fn herd_steps(elapsed: Duration, working: usize) -> Option<u64> {
    if working == 0 {
        return None;
    }
    let extra = f64::from(u32::try_from(working - 1).unwrap_or(u32::MAX));
    let speed = (SLOWEST_COLUMNS_PER_SECOND + extra).min(FASTEST_COLUMNS_PER_SECOND);
    // Truncation is the intent: whole columns walked so far.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = (elapsed.as_secs_f64() * speed) as u64;
    Some(steps)
}

fn blink_on(elapsed: Duration, period: Duration) -> bool {
    (elapsed.as_millis() / period.as_millis()).is_multiple_of(2)
}

/// Three rows of pixel art, `width` cells each.
pub(super) fn art(
    width: usize,
    elapsed: Duration,
    statuses: &[AgentStatus],
    palette: &Palette,
) -> Vec<Vec<HerdCell>> {
    let working = statuses
        .iter()
        .filter(|status| **status == AgentStatus::Working)
        .count();
    let mut herd = herd_order(statuses);
    if herd.is_empty() {
        // No agents yet: one sheep naps in the meadow.
        herd.push(AgentStatus::Idle);
    }
    let capacity = ((width + SHEEP_SPAN - SHEEP_WIDTH) / SHEEP_SPAN).max(1);
    herd.truncate(capacity);

    let steps = herd_steps(elapsed, working);
    let ring = (width + SHEEP_WIDTH).max(herd.len() * SHEEP_SPAN);
    let mut pixels = vec![vec![None::<Color>; width]; PIXEL_ROWS];
    let mut marks: Vec<(usize, HerdCell)> = Vec::new();

    for (index, status) in herd.iter().enumerate() {
        let base = index * SHEEP_SPAN + 1;
        let x = match steps {
            // usize -> u64 is lossless on every supported target.
            Some(steps) => {
                let position = usize::try_from((base as u64 + steps) % ring as u64).unwrap_or(0);
                if position >= width {
                    position.cast_signed() - ring.cast_signed()
                } else {
                    position.cast_signed()
                }
            }
            None => base.cast_signed(),
        };
        let pose = match (steps, status) {
            (Some(steps), _) => Pose::Walking {
                legs_apart: steps.is_multiple_of(2),
            },
            (None, AgentStatus::Blocked) => Pose::Standing,
            (None, _) => Pose::Sleeping,
        };
        let wool = wool_color(*status, palette);
        draw_sheep(&mut pixels, x, pose, wool, palette);

        let mark_column = |offset: isize| {
            usize::try_from(x + offset)
                .ok()
                .filter(|column| *column < width)
        };
        if *status == AgentStatus::Blocked && blink_on(elapsed, ALERT_BLINK) {
            if let Some(column) = mark_column(5) {
                marks.push((
                    column,
                    HerdCell {
                        ch: '!',
                        fg: Some(palette.red),
                        bg: None,
                        bold: true,
                    },
                ));
            }
        }
        if pose == Pose::Sleeping {
            if let Some(column) = mark_column(6) {
                let ch = if blink_on(elapsed, SNORE_BLINK) {
                    'z'
                } else {
                    'Z'
                };
                marks.push((
                    column,
                    HerdCell {
                        ch,
                        fg: Some(palette.overlay0),
                        bg: None,
                        bold: false,
                    },
                ));
            }
        }
    }

    let mut rows = (0..PIXEL_ROWS / 2)
        .map(|row| {
            (0..width)
                .map(|column| half_block(pixels[row * 2][column], pixels[row * 2 + 1][column]))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (column, cell) in marks {
        if rows[0][column] == EMPTY {
            rows[0][column] = cell;
        }
    }
    // Sparse ground dots scroll left while the herd walks.
    let ground_offset = usize::try_from(steps.unwrap_or(0) % GROUND_SPACING as u64).unwrap_or(0);
    for (column, cell) in rows[2].iter_mut().enumerate() {
        if *cell == EMPTY && (column + ground_offset) % GROUND_SPACING == 3 {
            *cell = HerdCell {
                ch: '·',
                fg: Some(palette.surface1),
                bg: None,
                bold: false,
            };
        }
    }
    rows
}

fn draw_sheep(
    pixels: &mut [Vec<Option<Color>>],
    x: isize,
    pose: Pose,
    wool: Color,
    palette: &Palette,
) {
    let (top, sprite): (usize, Vec<&str>) = match pose {
        Pose::Walking { legs_apart } => (
            1,
            BODY.iter()
                .chain(if legs_apart {
                    LEGS_APART.iter()
                } else {
                    LEGS_TOGETHER.iter()
                })
                .copied()
                .collect(),
        ),
        Pose::Standing => (1, BODY.iter().chain(LEGS_APART.iter()).copied().collect()),
        Pose::Sleeping => (1, SLEEPING.to_vec()),
    };
    for (dy, line) in sprite.iter().enumerate() {
        let Some(row) = pixels.get_mut(top + dy) else {
            continue;
        };
        for (dx, pixel) in line.chars().enumerate() {
            let color = match pixel {
                'W' => wool,
                'H' => palette.overlay1,
                'L' => palette.overlay0,
                _ => continue,
            };
            if let Some(column) = usize::try_from(x + dx.cast_signed())
                .ok()
                .filter(|column| *column < row.len())
            {
                row[column] = Some(color);
            }
        }
    }
}

fn half_block(top: Option<Color>, bottom: Option<Color>) -> HerdCell {
    let cell = |ch, fg, bg| HerdCell {
        ch,
        fg: Some(fg),
        bg,
        bold: false,
    };
    match (top, bottom) {
        (None, None) => EMPTY,
        (Some(top), None) => cell('▀', top, None),
        (None, Some(bottom)) => cell('▄', bottom, None),
        (Some(top), Some(bottom)) if top == bottom => cell('█', top, None),
        (Some(top), Some(bottom)) => cell('▀', top, Some(bottom)),
    }
}

/// `MoMo  ● 2 working  ▲ 1 needs you`, a compact `MoMo  ● 2  ▲ 1` when that
/// does not fit in `width` columns, or a quiet summary.
pub(super) fn status_line(
    statuses: &[AgentStatus],
    palette: &Palette,
    width: usize,
) -> Vec<(String, Style)> {
    let full = status_segments(statuses, palette, false);
    let length: usize = full.iter().map(|(text, _)| text.chars().count()).sum();
    if length <= width {
        full
    } else {
        status_segments(statuses, palette, true)
    }
}

fn status_segments(
    statuses: &[AgentStatus],
    palette: &Palette,
    compact: bool,
) -> Vec<(String, Style)> {
    let label = |long: &'static str| if compact { "" } else { long };
    let count = |wanted: AgentStatus| statuses.iter().filter(|status| **status == wanted).count();
    let (working, blocked, done) = (
        count(AgentStatus::Working),
        count(AgentStatus::Blocked),
        count(AgentStatus::Done),
    );
    let mut segments = vec![(
        format!(" {}", crate::brand::PRODUCT_NAME),
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
    )];
    if working > 0 {
        segments.push((
            format!("  ● {working}{}", label(" working")),
            Style::default().fg(palette.yellow),
        ));
    }
    if blocked > 0 {
        segments.push((
            format!("  ▲ {blocked}{}", label(" needs you")),
            Style::default()
                .fg(palette.red)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if working == 0 && blocked == 0 {
        segments.push(if done > 0 {
            (
                format!("  ✓ {done}{}", label(" done")),
                Style::default().fg(palette.teal),
            )
        } else {
            (
                "  · all quiet".to_owned(),
                Style::default().fg(palette.overlay0),
            )
        });
    }
    segments
}

/// Draw the banner into `area` (the top `BANNER_ROWS` rows of the sidebar).
pub(super) fn render(
    buffer: &mut Buffer,
    area: Rect,
    elapsed: Duration,
    statuses: &[AgentStatus],
    palette: &Palette,
) {
    super::render::render_sidebar_background(buffer, area, palette);
    // The rightmost column is the sidebar separator.
    let width = usize::from(area.width.saturating_sub(1));
    for (dy, row) in (0u16..).zip(art(width, elapsed, statuses, palette)) {
        for (dx, cell) in (0u16..).zip(row) {
            if cell == EMPTY {
                continue;
            }
            let mut style = Style::default();
            if let Some(fg) = cell.fg {
                style = style.fg(fg);
            }
            if let Some(bg) = cell.bg {
                style = style.bg(bg);
            }
            if cell.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if let Some(target) = buffer.cell_mut((area.x + dx, area.y + dy)) {
                target.set_char(cell.ch).set_style(style);
            }
        }
    }
    let mut x = area.x;
    let right = area.x + area.width.saturating_sub(1);
    for (text, style) in status_line(statuses, palette, width) {
        let available = right.saturating_sub(x);
        if available == 0 {
            break;
        }
        super::render::put_text(buffer, x, area.y + 3, available, &text, style);
        x = x.saturating_add(u16::try_from(text.chars().count()).unwrap_or(u16::MAX));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        Palette::midnight_neon()
    }

    fn text(rows: &[Vec<HerdCell>]) -> Vec<String> {
        rows.iter()
            .map(|row| row.iter().map(|cell| cell.ch).collect())
            .collect()
    }

    fn colors(rows: &[Vec<HerdCell>]) -> Vec<Color> {
        rows.iter().flatten().filter_map(|cell| cell.fg).collect()
    }

    #[test]
    fn frames_are_deterministic_and_never_exceed_the_width() {
        let statuses = [
            AgentStatus::Working,
            AgentStatus::Blocked,
            AgentStatus::Idle,
        ];
        for width in [0, 1, 7, 12, 30, 44] {
            for millis in [0, 130, 999, 12_345, 3_600_000] {
                let elapsed = Duration::from_millis(millis);
                let first = art(width, elapsed, &statuses, &palette());
                assert_eq!(first, art(width, elapsed, &statuses, &palette()));
                assert_eq!(first.len(), 3);
                assert!(first.iter().all(|row| row.len() == width));
            }
        }
    }

    #[test]
    fn wool_shows_each_agents_state_and_needs_you_sheep_come_first() {
        let palette = palette();
        let rows = art(
            12,
            Duration::ZERO,
            &[AgentStatus::Idle, AgentStatus::Blocked],
            &palette,
        );
        // Only one sheep fits in 12 columns: the one that needs you.
        let colors = colors(&rows);
        assert!(colors.contains(&palette.red));
        assert!(!colors.contains(&palette.green));

        let wide = art(
            40,
            Duration::ZERO,
            &[AgentStatus::Working, AgentStatus::Done, AgentStatus::Idle],
            &palette,
        );
        let colors = self::colors(&wide);
        for state in [palette.yellow, palette.teal, palette.green] {
            assert!(colors.contains(&state), "missing {state:?}");
        }
    }

    #[test]
    fn the_herd_walks_faster_with_more_working_agents() {
        let one = herd_steps(Duration::from_secs(10), 1).unwrap();
        let three = herd_steps(Duration::from_secs(10), 3).unwrap();
        let many = herd_steps(Duration::from_secs(10), 50).unwrap();
        assert_eq!(one, 30);
        assert_eq!(three, 50);
        assert_eq!(many, 80, "speed is capped");
        assert!(herd_steps(Duration::from_secs(10), 0).is_none());

        let statuses = [AgentStatus::Working];
        let before = text(&art(30, Duration::ZERO, &statuses, &palette()));
        let after = text(&art(30, Duration::from_millis(400), &statuses, &palette()));
        assert_ne!(before, after, "a working herd moves");
    }

    #[test]
    fn legs_alternate_between_steps() {
        let statuses = [AgentStatus::Working];
        let legs = |millis| {
            text(&art(
                30,
                Duration::from_millis(millis),
                &statuses,
                &palette(),
            ))[2]
                .replace('·', " ")
                .trim()
                .to_owned()
        };
        // 3 columns/second: one step every ~333 ms.
        assert_eq!(legs(0), "█  █");
        assert_eq!(legs(340), "██");
    }

    #[test]
    fn needs_you_blinks_and_idle_herds_sleep() {
        let statuses = [AgentStatus::Blocked, AgentStatus::Idle];
        let lit = text(&art(30, Duration::ZERO, &statuses, &palette()));
        let dark = text(&art(30, ALERT_BLINK, &statuses, &palette()));
        assert!(lit[0].contains('!'));
        assert!(!dark[0].contains('!'));
        // Nothing works: the idle sheep sleeps and snores.
        assert!(lit[0].contains('z'));
        assert!(text(&art(30, SNORE_BLINK, &statuses, &palette()))[0].contains('Z'));

        let empty = text(&art(30, Duration::ZERO, &[], &palette()));
        assert!(empty[0].contains('z'), "a lone sheep naps without agents");
    }

    #[test]
    fn status_line_counts_what_matters() {
        let palette = palette();
        let line = |statuses: &[AgentStatus]| {
            status_line(statuses, &palette, 60)
                .into_iter()
                .map(|(text, _)| text)
                .collect::<String>()
        };
        assert_eq!(
            line(&[
                AgentStatus::Working,
                AgentStatus::Working,
                AgentStatus::Blocked
            ]),
            " MoMo  ● 2 working  ▲ 1 needs you"
        );
        assert_eq!(line(&[AgentStatus::Done]), " MoMo  ✓ 1 done");
        assert_eq!(line(&[AgentStatus::Idle]), " MoMo  · all quiet");

        // A narrow sidebar keeps every count visible instead of cutting words.
        let narrow = status_line(
            &[
                AgentStatus::Working,
                AgentStatus::Working,
                AgentStatus::Blocked,
            ],
            &palette,
            25,
        )
        .into_iter()
        .map(|(text, _)| text)
        .collect::<String>();
        assert_eq!(narrow, " MoMo  ● 2  ▲ 1");
    }

    #[test]
    fn banner_only_fits_tall_enough_sidebars() {
        assert!(fits(Rect::new(0, 0, 26, 16)));
        assert!(!fits(Rect::new(0, 0, 26, 15)));
        assert!(!fits(Rect::new(0, 0, 11, 40)));
    }
}
