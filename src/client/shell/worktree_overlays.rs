use super::*;

pub(super) fn render_worktree_create_overlay(
    b: &mut Buffer,
    create: &ClientWorktreeCreateOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let popup = popup(b.area, 68, 12)?;
    let inner = panel(b, popup, p.accent, p.panel_bg)?;
    put_text(
        b,
        inner.x,
        inner.y,
        inner.width,
        "new worktree",
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    put_text(
        b,
        inner.x,
        inner.y + 2,
        inner.width,
        " branch",
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    let input = Rect::new(inner.x, inner.y + 3, inner.width, 1);
    b.set_style(input, Style::default().fg(p.text).bg(p.surface0));
    let cursor = text_editor::render(
        b,
        Rect::new(input.x + 1, input.y, input.width.saturating_sub(1), 1),
        &create.branch,
        Style::default().fg(p.text).bg(p.surface0),
    );
    put_text(
        b,
        inner.x,
        inner.y + 5,
        inner.width,
        " checkout",
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    put_text(
        b,
        inner.x,
        inner.y + 6,
        inner.width,
        &format!(" {}", create.checkout_path),
        Style::default().fg(p.subtext0).bg(p.panel_bg),
    );
    if create.creating {
        put_text(
            b,
            inner.x,
            inner.y + 8,
            inner.width,
            " creating…",
            Style::default().fg(p.accent).bg(p.panel_bg),
        );
    } else if let Some(error) = create.error.as_deref() {
        put_text(
            b,
            inner.x,
            inner.y + 8,
            inner.width,
            &format!(" {error}"),
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    let buttons = row(inner, &[20, 12], 2, 9);
    let [primary, cancel] = buttons.as_slice() else {
        return None;
    };
    button(
        b,
        *primary,
        " ↵ create and open ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        area: popup,
        primary: *primary,
        clear: Rect::default(),
        cancel: *cancel,
        navigator_popup: Rect::default(),
        navigator_search: Rect::default(),
        navigator_rows: Vec::new(),
        worktree_search: Rect::default(),
        worktree_rows: Vec::new(),
        cursor: cursor.filter(|_| !create.creating),
        ..OverlayRender::default()
    })
}

pub(super) fn render_worktree_open_overlay(
    b: &mut Buffer,
    open: &ClientWorktreeOpenOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let popup_height = (open.entries.len().saturating_mul(2) + 7).clamp(12, 26) as u16;
    let popup = popup(b.area, 96, popup_height)?;
    let inner = panel(b, popup, p.accent, p.panel_bg)?;
    put_text(
        b,
        inner.x,
        inner.y,
        inner.width,
        "open worktree",
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    let search = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    let filtered = open.filtered_indices();
    put_text(
        b,
        search.x,
        search.y,
        search.width,
        &if open.search_focused {
            " / ".to_owned()
        } else if !open.query.is_empty() {
            format!(" / {}", open.query)
        } else {
            " / filter worktrees".to_owned()
        },
        Style::default()
            .fg(if open.search_focused {
                p.text
            } else {
                p.overlay0
            })
            .bg(p.panel_bg),
    );
    let count = if filtered.len() == open.entries.len() {
        format!("{} checkouts", open.entries.len())
    } else {
        format!("{}/{} checkouts", filtered.len(), open.entries.len())
    };
    let cursor = if open.search_focused {
        text_editor::render(
            b,
            Rect::new(
                search.x + 3,
                search.y,
                search.width.saturating_sub(4 + display_width(&count)),
                1,
            ),
            &open.query,
            Style::default().fg(p.text).bg(p.panel_bg),
        )
    } else {
        None
    };
    put_right_text(
        b,
        search,
        search.y,
        &count,
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    put_text(
        b,
        inner.x,
        inner.y + 2,
        inner.width,
        &"─".repeat(inner.width as usize),
        Style::default().fg(p.surface1).bg(p.panel_bg),
    );
    let body = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(6),
    );
    let visible_count = (body.height / 2).max(1) as usize;
    let selected_position = filtered
        .iter()
        .position(|index| *index == open.selected)
        .unwrap_or(0);
    let start = selected_position
        .saturating_sub(visible_count.saturating_sub(1))
        .min(filtered.len().saturating_sub(visible_count));
    let mut row_hits = Vec::new();
    for (visible, entry_index) in filtered
        .iter()
        .copied()
        .skip(start)
        .take(visible_count)
        .enumerate()
    {
        let entry = &open.entries[entry_index];
        let rect = Rect::new(body.x, body.y + visible as u16 * 2, body.width, 2);
        row_hits.push((rect, entry_index));
        let selected = entry_index == open.selected;
        let style = if selected {
            Style::default().fg(contrast(p)).bg(p.accent)
        } else {
            Style::default().fg(p.text).bg(p.panel_bg)
        };
        b.set_style(rect, style);
        put_text(
            b,
            rect.x,
            rect.y,
            rect.width,
            &format!(" {}", entry.label),
            style.add_modifier(Modifier::BOLD),
        );
        let status = entry.status_label();
        if !status.is_empty() {
            put_right_text(b, rect, rect.y, status, style);
        }
        put_text(
            b,
            rect.x,
            rect.y + 1,
            rect.width,
            &format!(" {}", entry.path),
            if selected {
                style
            } else {
                Style::default().fg(p.overlay0).bg(p.panel_bg)
            },
        );
    }
    if filtered.is_empty() {
        put_text(
            b,
            body.x,
            body.y,
            body.width,
            " no matching worktrees",
            Style::default().fg(p.overlay0).bg(p.panel_bg),
        );
    }
    if open.opening {
        put_text(
            b,
            inner.x,
            inner.bottom() - 3,
            inner.width,
            " opening…",
            Style::default().fg(p.accent).bg(p.panel_bg),
        );
    } else if let Some(error) = open.error.as_deref() {
        put_text(
            b,
            inner.x,
            inner.bottom() - 3,
            inner.width,
            &format!(" {error}"),
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    let buttons = row(inner, &[10, 12], 2, inner.height.saturating_sub(1));
    let [primary, cancel] = buttons.as_slice() else {
        return None;
    };
    button(
        b,
        *primary,
        " ↵ open ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        area: popup,
        primary: *primary,
        clear: Rect::default(),
        cancel: *cancel,
        navigator_popup: Rect::default(),
        navigator_search: Rect::default(),
        navigator_rows: Vec::new(),
        worktree_search: search,
        worktree_rows: row_hits,
        cursor: cursor.filter(|_| !open.opening),
        ..OverlayRender::default()
    })
}

pub(super) fn render_worktree_remove_overlay(
    b: &mut Buffer,
    remove: &ClientWorktreeRemoveOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let nested_lines = nested_risk_lines(remove);
    let extra = u16::try_from(nested_lines.len()).unwrap_or(u16::MAX);
    let popup = popup(b.area, 72, 10u16.saturating_add(extra))?;
    let inner = panel(b, popup, p.red, p.panel_bg)?;
    put_text(
        b,
        inner.x,
        inner.y,
        inner.width,
        " delete worktree checkout?",
        Style::default()
            .fg(p.red)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    put_text(
        b,
        inner.x,
        inner.y + 1,
        inner.width,
        " This removes the checkout folder:",
        Style::default().fg(p.text).bg(p.panel_bg),
    );
    put_text(
        b,
        inner.x,
        inner.y + 2,
        inner.width,
        &format!(" {}", remove.path),
        Style::default().fg(p.subtext0).bg(p.panel_bg),
    );
    put_text(
        b,
        inner.x,
        inner.y + 3,
        inner.width,
        " The branch is not deleted. The MoMo workspace will close.",
        Style::default().fg(p.text).bg(p.panel_bg),
    );
    if remove.force_confirmation {
        put_text(
            b,
            inner.x,
            inner.y + 4,
            inner.width,
            " Dirty or untracked files will be permanently deleted.",
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    for (offset, line) in (0u16..).zip(&nested_lines) {
        put_text(
            b,
            inner.x,
            inner.y.saturating_add(5).saturating_add(offset),
            inner.width,
            line,
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    let status_row = inner.y.saturating_add(5).saturating_add(extra);
    if remove.removing {
        put_text(
            b,
            inner.x,
            status_row,
            inner.width,
            " removing…",
            Style::default().fg(p.accent).bg(p.panel_bg),
        );
    } else if let Some(error) = remove.error.as_deref() {
        put_text(
            b,
            inner.x,
            status_row,
            inner.width,
            &format!(" {error}"),
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    let primary_label = if remove.nested.is_some() {
        " ↵ discard nested work "
    } else if remove.force_confirmation {
        " ↵ delete anyway "
    } else {
        " ↵ remove "
    };
    let primary_width = u16::try_from(primary_label.chars().count())
        .unwrap_or(18)
        .max(18);
    let buttons = row(inner, &[primary_width, 12], 2, 7u16.saturating_add(extra));
    let [primary, cancel] = buttons.as_slice() else {
        return None;
    };
    button(
        b,
        *primary,
        primary_label,
        Style::default()
            .fg(contrast(p))
            .bg(p.red)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        area: popup,
        primary: *primary,
        clear: Rect::default(),
        cancel: *cancel,
        navigator_popup: Rect::default(),
        navigator_search: Rect::default(),
        navigator_rows: Vec::new(),
        worktree_search: Rect::default(),
        worktree_rows: Vec::new(),
        cursor: None,
        ..OverlayRender::default()
    })
}

/// Warning rows for nested repositories that discarding would delete: a heading,
/// up to three repositories, and a count of the rest.
fn nested_risk_lines(remove: &ClientWorktreeRemoveOverlay) -> Vec<String> {
    const SHOWN: usize = 3;
    let Some(nested) = remove.nested.as_ref() else {
        return Vec::new();
    };
    let mut lines = vec![if nested.is_empty() {
        " Too large to verify nested repositories; they may hold unsaved work.".to_owned()
    } else {
        " Nested repositories with work that will be lost:".to_owned()
    }];
    for risk in nested.iter().take(SHOWN) {
        lines.push(format!("   {}: {}", risk.relative_path, risk.summary));
    }
    if nested.len() > SHOWN {
        lines.push(format!("   …and {} more", nested.len() - SHOWN));
    }
    if !nested.is_empty() && !remove.nested_check_complete {
        lines.push("   (scan stopped early; more may be at risk)".to_owned());
    }
    lines
}
