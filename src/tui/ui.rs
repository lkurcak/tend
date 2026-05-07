use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap},
};

use crate::tui::app::{App, GroupListItem, JobStatus, OutputStream, Tab};

pub fn draw(f: &mut Frame<'_>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Footer with keybindings
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame<'_>, app: &App, area: Rect) {
    let titles: Vec<Line<'_>> = vec!["Jobs [1]".into(), "Groups [2]".into()];
    let selected = match app.tab {
        Tab::Jobs => 0,
        Tab::Groups => 1,
    };
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" tend v0.3.0 "),
        )
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );
    f.render_widget(tabs, area);
}

fn draw_main(f: &mut Frame<'_>, app: &mut App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_chunks[0]);

    match app.tab {
        Tab::Jobs => draw_jobs_list(f, app, left_chunks[0]),
        Tab::Groups => draw_groups_list(f, app, left_chunks[0]),
    }

    draw_events(f, app, left_chunks[1]);
    draw_output(f, app, main_chunks[1]);
}

const fn status_icon(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Running => "●",
        JobStatus::Stopped => "○",
        JobStatus::Restarting => "◌",
    }
}

const fn status_color(status: JobStatus) -> Color {
    match status {
        JobStatus::Running => Color::Green,
        JobStatus::Stopped => Color::DarkGray,
        JobStatus::Restarting => Color::Yellow,
    }
}

fn draw_jobs_list(f: &mut Frame<'_>, app: &mut App, area: Rect) {
    let rows: Vec<Row<'_>> = app
        .jobs
        .iter()
        .map(|job_info| {
            let icon = status_icon(job_info.status);
            let color = status_color(job_info.status);
            Row::new(vec![
                Cell::from(Span::styled(format!(" {icon}"), Style::default().fg(color))),
                Cell::from(Span::styled(
                    job_info.job.name.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    job_info.status.to_string(),
                    Style::default().fg(color),
                )),
                Cell::from(
                    Text::styled(job_info.uptime_str(), Style::default().fg(Color::Yellow))
                        .right_aligned(),
                ),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .block(Block::default().borders(Borders::ALL).title(" Jobs "))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▸ ");

    f.render_stateful_widget(table, area, &mut app.job_list_state);
}

fn draw_groups_list(f: &mut Frame<'_>, app: &mut App, area: Rect) {
    let rows: Vec<Row<'_>> = app
        .group_list_items
        .iter()
        .map(|item| match item {
            GroupListItem::Group(gi) => {
                let group = &app.groups[*gi];
                let (running, total) = app.group_job_counts(&group.name);
                let icon = if group.expanded { "▼" } else { "▶" };
                let count_color = if running == total && total > 0 {
                    Color::Green
                } else if running > 0 {
                    Color::Yellow
                } else {
                    Color::DarkGray
                };
                Row::new(vec![
                    Cell::from(Span::styled(
                        format!(" {icon}"),
                        Style::default().fg(Color::White),
                    )),
                    Cell::from(Span::styled(
                        group.name.clone(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(Span::styled(
                        format!("{running}/{total}"),
                        Style::default().fg(count_color),
                    )),
                    Cell::from(""),
                ])
            }
            GroupListItem::Job(ji) => {
                let job_info = &app.jobs[*ji];
                let icon = status_icon(job_info.status);
                let color = status_color(job_info.status);
                Row::new(vec![
                    Cell::from(Span::styled(
                        format!("   {icon}"),
                        Style::default().fg(color),
                    )),
                    Cell::from(Span::styled(
                        job_info.job.name.clone(),
                        Style::default().fg(Color::Cyan),
                    )),
                    Cell::from(Span::styled(
                        job_info.status.to_string(),
                        Style::default().fg(color),
                    )),
                    Cell::from(
                        Text::styled(job_info.uptime_str(), Style::default().fg(Color::Yellow))
                            .right_aligned(),
                    ),
                ])
            }
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .block(Block::default().borders(Borders::ALL).title(" Groups "))
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▸ ");

    f.render_stateful_widget(table, area, &mut app.group_list_state);
}

#[allow(clippy::option_if_let_else)]
fn draw_events(f: &mut Frame<'_>, app: &App, area: Rect) {
    let selected_idx = app.selected_job_index();
    let title = if let Some(idx) = selected_idx {
        format!(" Events: {} ", app.jobs[idx].job.name)
    } else if let Some(gi) = app.selected_group_index() {
        format!(" Events: {} ", app.groups[gi].name)
    } else {
        " Events ".to_string()
    };

    let events: Vec<Line<'_>> = if let Some(idx) = selected_idx {
        app.jobs[idx]
            .events
            .iter()
            .map(|e| {
                Line::from(Span::styled(
                    e.message.to_string(),
                    Style::default().fg(Color::White),
                ))
            })
            .collect()
    } else {
        vec![]
    };

    let content_height = area.height.saturating_sub(2); // borders
    let total_events = events.len();
    #[allow(clippy::cast_possible_truncation)]
    let scroll = total_events.saturating_sub(content_height as usize) as u16;

    let events_paragraph = Paragraph::new(events)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(events_paragraph, area);
}

#[allow(clippy::option_if_let_else, clippy::cast_possible_truncation)]
fn draw_output(f: &mut Frame<'_>, app: &App, area: Rect) {
    let selected_idx = app.selected_job_index();
    let title = if let Some(idx) = selected_idx {
        format!(" Output: {} ", app.jobs[idx].job.name)
    } else {
        " Output ".to_string()
    };

    let output_lines: Vec<Line<'_>> = if let Some(idx) = selected_idx {
        app.jobs[idx]
            .output
            .iter()
            .map(|line| {
                let style = match line.stream {
                    OutputStream::Stdout => Style::default().fg(Color::White),
                    OutputStream::Stderr => Style::default().fg(Color::Red),
                    OutputStream::System => Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::ITALIC),
                };
                Line::from(Span::styled(line.content.to_string(), style))
            })
            .collect()
    } else {
        vec![Line::from(Span::styled(
            "Select a job to view output",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let content_height = area.height.saturating_sub(2);
    let total_lines = output_lines.len();
    let max_scroll = total_lines.saturating_sub(content_height as usize) as u16;
    let scroll = max_scroll.saturating_sub(app.output_scroll_offset);

    let output = Paragraph::new(output_lines)
        .block(Block::default().borders(Borders::ALL).title(title))
        .scroll((scroll, 0));

    f.render_widget(output, area);
}

fn draw_footer(f: &mut Frame<'_>, app: &App, area: Rect) {
    let keybind = |key: &str, desc: &str| -> Vec<Span<'_>> {
        vec![
            Span::styled(
                format!(" {key}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(":{desc}"), Style::default().fg(Color::White)),
        ]
    };

    let mut spans: Vec<Span<'_>> = Vec::new();
    spans.extend(keybind("q", "Quit"));
    spans.extend(keybind("Tab", "Switch"));
    spans.extend(keybind("jk", "Navigate"));

    match app.tab {
        Tab::Jobs => {
            spans.extend(keybind("s", "Start"));
            spans.extend(keybind("x", "Stop"));
            spans.extend(keybind("r", "Restart"));
            spans.extend(keybind("l", "Logs"));
            spans.extend(keybind("PgUp/Dn", "Scroll"));
        }
        Tab::Groups => {
            spans.extend(keybind("Enter", "Expand"));
            spans.extend(keybind("s", "Start"));
            spans.extend(keybind("x", "Stop"));
            spans.extend(keybind("r", "Restart"));
            spans.extend(keybind("l", "Logs"));
            spans.extend(keybind("PgUp/Dn", "Scroll"));
        }
    }

    let footer = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::DarkGray));

    f.render_widget(footer, area);
}
