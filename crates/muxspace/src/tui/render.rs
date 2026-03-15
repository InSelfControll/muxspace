use crate::ansi::{Cell, Style};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style as RStyle},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Clear},
    Frame,
};

use super::layout::compute_pane_areas;
use super::{AppState, AppMode};

pub fn draw(f: &mut Frame, app: &AppState) {
    match app.mode {
        AppMode::WorkspaceView => draw_workspace_view(f, app),
        AppMode::ProjectNavigator => draw_project_navigator(f, app),
        AppMode::SearchMode => {
            draw_workspace_view(f, app);
            draw_search_overlay(f, app);
        }
    }
}

fn draw_workspace_view(f: &mut Frame, app: &AppState) {
    let area = f.size();

    // ── Tab bar ───────────────────────────────────────────────────────────────
    let tab_titles: Vec<Line> = app
        .workspace_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let s = if i == app.active_workspace {
                RStyle::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                RStyle::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(name.as_str(), s))
        })
        .collect();

    let tab_area = Rect { height: 2, ..area };
    f.render_widget(
        Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::BOTTOM))
            .select(app.active_workspace),
        tab_area,
    );

    // ── Pane grid ─────────────────────────────────────────────────────────────
    let pane_area = Rect {
        y: area.y + 2,
        height: area.height.saturating_sub(2),
        ..area
    };

    let pane_areas = compute_pane_areas(pane_area, app.panes.len());

    for pa in &pane_areas {
        let pane = &app.panes[pa.index];
        let is_active = pa.index == app.active_pane;

        let border_style = if is_active {
            RStyle::default().fg(Color::Cyan)
        } else {
            RStyle::default().fg(Color::DarkGray)
        };

        let prefix = if app.prefix_mode && is_active { "⌨ " } else { "" };
        let search_info = if is_active && !pane.search_matches.is_empty() {
            format!(" [{}/{}]", pane.current_match + 1, pane.search_matches.len())
        } else {
            String::new()
        };
        let title = format!(" {}{}{} ", prefix, pane.title, search_info);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(pa.rect);
        f.render_widget(block, pa.rect);
        render_pane(f, pane, inner, is_active);
    }
}

fn render_pane(f: &mut Frame, pane: &super::PaneState, area: Rect, is_active: bool) {
    let screen = pane.screen.lock().unwrap();
    let h = area.height as usize;
    let w = area.width as usize;

    // Combine scrollback + grid into one slice, then take the visible window.
    let total_rows = screen.scrollback.len() + screen.rows;
    let offset = pane.scroll_offset.min(screen.scrollback.len());
    let view_start = total_rows.saturating_sub(h + offset);

    let lines: Vec<Line> = (view_start..view_start + h)
        .map(|row_idx| {
            let row: &[Cell] = if row_idx < screen.scrollback.len() {
                &screen.scrollback[row_idx]
            } else {
                let grid_row = row_idx - screen.scrollback.len();
                if grid_row < screen.grid.len() { &screen.grid[grid_row] } else { &[] }
            };
            
            // Check if this row has search matches
            let _line_text: String = row.iter().map(|c| c.ch).collect();
            let mut spans = Vec::new();
            
            if is_active && !pane.search_query.is_empty() {
                // Highlight search matches
                let mut last_end = 0;
                for (match_row, match_col) in &pane.search_matches {
                    if *match_row == row_idx && *match_col < row.len() {
                        // Add text before match
                        if *match_col > last_end {
                            spans.extend(row[last_end..*match_col].iter().map(|cell| {
                                Span::styled(cell.ch.to_string(), cell_style(&cell.style))
                            }));
                        }
                        // Add highlighted match
                        let match_end = (*match_col + pane.search_query.len()).min(row.len());
                        for (i, cell) in row[*match_col..match_end].iter().enumerate() {
                            let is_current = pane.search_matches.get(pane.current_match) == Some(&(*match_row, *match_col + i));
                            let style = if is_current {
                                RStyle::default().bg(Color::Yellow).fg(Color::Black)
                            } else {
                                RStyle::default().bg(Color::DarkGray).fg(Color::White)
                            };
                            spans.push(Span::styled(cell.ch.to_string(), style));
                        }
                        last_end = match_end;
                    }
                }
                // Add remaining text
                if last_end < row.len() {
                    spans.extend(row[last_end..].iter().map(|cell| {
                        Span::styled(cell.ch.to_string(), cell_style(&cell.style))
                    }));
                }
            } else {
                spans = row
                    .iter()
                    .take(w)
                    .map(|cell| Span::styled(cell.ch.to_string(), cell_style(&cell.style)))
                    .collect();
            }
            
            Line::from(spans)
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}

fn cell_style(s: &Style) -> RStyle {
    let mut rs = RStyle::default();
    if let Some(fg) = s.fg { rs = rs.fg(ansi_color(fg)); }
    if let Some(bg) = s.bg { rs = rs.bg(ansi_color(bg)); }
    if s.bold      { rs = rs.add_modifier(Modifier::BOLD); }
    if s.italic    { rs = rs.add_modifier(Modifier::ITALIC); }
    if s.underline { rs = rs.add_modifier(Modifier::UNDERLINED); }
    if s.reversed  { rs = rs.add_modifier(Modifier::REVERSED); }
    rs
}

fn ansi_color(idx: u8) -> Color {
    match idx {
        0  => Color::Black,
        1  => Color::Red,
        2  => Color::Green,
        3  => Color::Yellow,
        4  => Color::Blue,
        5  => Color::Magenta,
        6  => Color::Cyan,
        7  => Color::Gray,
        8  => Color::DarkGray,
        9  => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        15 => Color::White,
        n  => Color::Indexed(n),
    }
}

// ── Project Navigator ─────────────────────────────────────────────────────────

fn draw_project_navigator(f: &mut Frame, app: &AppState) {
    let area = f.size();

    // Full-screen overlay with border
    let block = Block::default()
        .title(" Project Navigator ")
        .borders(Borders::ALL)
        .border_style(RStyle::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split into list and preview
    let list_width = (inner.width / 3).max(30);
    let list_area = Rect {
        width: list_width,
        height: inner.height,
        ..inner
    };
    let preview_area = Rect {
        x: inner.x + list_width,
        y: inner.y,
        width: inner.width - list_width,
        height: inner.height,
    };

    // Draw project list
    draw_project_list(f, app, list_area);

    // Draw preview
    draw_project_preview(f, app, preview_area);
}

fn draw_project_list(f: &mut Frame, app: &AppState, area: Rect) {
    let items: Vec<Line> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, proj)| {
            let style = if i == app.selected_project {
                RStyle::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                RStyle::default().fg(Color::White)
            };
            let workspace_count = proj.workspaces.len();
            let text = format!(" {} ({}) ", proj.name, workspace_count);
            Line::from(Span::styled(text, style))
        })
        .collect();

    let block = Block::default()
        .title(" Projects (j/k to navigate, Enter to select) ")
        .borders(Borders::RIGHT);

    f.render_widget(
        Paragraph::new(items).block(block),
        area,
    );
}

fn draw_project_preview(f: &mut Frame, app: &AppState, area: Rect) {
    let content = if let Some(proj) = app.projects.get(app.selected_project) {
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Project: {}", proj.name),
                RStyle::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)
            )),
            Line::from(""),
            Line::from("  Workspaces:"),
        ];
        for ws in &proj.workspaces {
            lines.push(Line::from(format!("    • {}", ws)));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press Enter to switch to this project",
            RStyle::default().fg(Color::DarkGray)
        )));
        lines
    } else {
        vec![Line::from("No projects available")]
    };

    let block = Block::default().title(" Preview ");
    f.render_widget(
        Paragraph::new(content).block(block),
        area,
    );
}

// ── Search Overlay ────────────────────────────────────────────────────────────

fn draw_search_overlay(f: &mut Frame, app: &AppState) {
    let area = f.size();
    
    // Centered search box
    let width = 50;
    let height = 3;
    let x = (area.width - width) / 2;
    let y = area.height / 3;
    
    let search_area = Rect { x, y, width, height };
    
    // Clear background
    f.render_widget(Clear, search_area);
    
    let block = Block::default()
        .title(" Search (ESC to cancel, TAB for next) ")
        .borders(Borders::ALL)
        .border_style(RStyle::default().fg(Color::Yellow));
    
    let inner = block.inner(search_area);
    f.render_widget(block, search_area);
    
    // Show search query
    let query = if app.search_query.is_empty() {
        Span::styled("Type to search...", RStyle::default().fg(Color::DarkGray))
    } else {
        Span::styled(&app.search_query, RStyle::default().fg(Color::White))
    };
    
    f.render_widget(
        Paragraph::new(Line::from(query)),
        inner,
    );
}
