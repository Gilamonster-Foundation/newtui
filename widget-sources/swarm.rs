// Copyright 2026 The Gilamonster Authors
// SPDX-License-Identifier: Apache-2.0

//! Swarm tab — 3x2 grid layout:
//! ```text
//! ┌─ Machines ────┐┌─ Repo Status ┐┌─ Budget ─────┐
//! │ gnuc CPU/MEM  ││ repo: gilabot││ Daily: $X    │
//! │ nuc  CPU/MEM  ││ Commits ░░█░ ││ Weekly: $X   │
//! │ NET butterfly ││ Python  ██░░ ││ Monthly: $X  │
//! ├─ gnuc PROC ──┐├─ nuc POD ───┤├─ nuc2 POD ───┤
//! │ PID USER ... ││ NS POD ... ││ NS POD ...   │
//! └──────────────┘└─────────────┘└──────────────┘
//! ```

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Direction;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, MachineState, ProcFilter};
use crate::ui::metrics::{draw_heat_meter, format_bytes, value_color};
use crate::ui::budget;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    // If pipeline failures exist, split off an alert bar at the top
    let (alert_area, grid_area) = if !app.pipeline_failures.is_empty() {
        let alert_height = (app.pipeline_failures.len() as u16 + 2).min(6);
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(alert_height), Constraint::Min(0)])
            .split(area);
        (Some(split[0]), split[1])
    } else {
        (None, area)
    };

    if let Some(alert_area) = alert_area {
        draw_pipeline_alerts(frame, app, alert_area);
    }

    // 3x2 grid
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(grid_area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(rows[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(rows[1]);

    // Top row: Machines | Repo Status | Budget
    draw_mini_metrics(frame, app, top[0]);
    draw_repo_status(frame, app, top[1]);
    budget::draw(frame, app, top[2]);

    // Bottom row: one proc/pod panel per machine
    for (i, bottom_area) in bottom.iter().enumerate() {
        if let Some(ms) = app.machine_states.get(i) {
            draw_mini_proc(frame, app, ms, *bottom_area);
        }
    }
}

/// Mini metrics overview — one row per machine with CPU/MEM HeatMeters + NET butterfly.
fn draw_mini_metrics(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Machines ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.metrics.machines.is_empty() {
        let msg = Paragraph::new("  no machine data")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    let rows_per_machine = 4;
    let constraints: Vec<Constraint> = app
        .metrics
        .machines
        .iter()
        .flat_map(|_| {
            vec![
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
        })
        .chain(std::iter::once(Constraint::Min(0)))
        .collect();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, m) in app.metrics.machines.iter().enumerate() {
        let base = i * rows_per_machine;
        if base + 3 >= rows.len() {
            break;
        }

        let connected = app
            .machine_states
            .get(i)
            .map(|ms| ms.connected)
            .unwrap_or(false);
        let status_color = if connected { Color::Green } else { Color::DarkGray };
        let cpu_color = value_color(m.cpu_percent / 100.0);
        let mem_color = value_color(m.memory_percent / 100.0);

        let label = Line::from(vec![
            Span::styled(
                format!(" {} ", m.name),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("CPU:{:.0}% ", m.cpu_percent),
                Style::default().fg(cpu_color),
            ),
            Span::styled(
                format!("MEM:{:.0}%", m.memory_percent),
                Style::default().fg(mem_color),
            ),
        ]);
        frame.render_widget(Paragraph::new(label), rows[base]);

        draw_heat_meter(frame, "CPU", m.cpu_percent, "", rows[base + 1]);
        draw_heat_meter(frame, "MEM", m.memory_percent, "", rows[base + 2]);
        draw_net_butterfly_meter(
            frame,
            m.net_tx_bytes_sec,
            m.net_rx_bytes_sec,
            app.net_label_width,
            rows[base + 3],
        );
    }
}

/// Repo status pane — shows one repo at a time with [/] scrolling.
///
/// Each repo shows a commit heatmap row plus one row per auto-discovered
/// workflow. Each hour-slot is a colored glyph: green=pass, red=fail,
/// yellow=in_progress, dark gray=no run.
fn draw_repo_status(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Repo Status ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.repo_snapshots.is_empty() {
        let msg = Paragraph::new("  polling...")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    let snap = match app.repo_snapshots.get(app.active_repo_idx) {
        Some(s) => s,
        None => return,
    };

    // Layout: header(1) + commits(1) + one per workflow + absorb
    let num_wf = snap.workflows.len();
    let mut constraints: Vec<Constraint> = Vec::new();
    constraints.push(Constraint::Length(1)); // header
    constraints.push(Constraint::Length(1)); // commits row
    for _ in 0..num_wf {
        constraints.push(Constraint::Length(1)); // workflow row
    }
    constraints.push(Constraint::Min(0)); // absorb

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let label_width: usize = 10;
    let bar_width = (inner.width as usize).saturating_sub(label_width);

    // Header: "repo: gilabot    [/]"
    let nav_hint = if app.repo_snapshots.len() > 1 {
        format!(
            " {}/{} [/]",
            app.active_repo_idx + 1,
            app.repo_snapshots.len()
        )
    } else {
        String::new()
    };
    let header = Line::from(vec![
        Span::styled(" repo: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            &snap.short_name,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(nav_hint, Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(header), rows[0]);

    // Commits row — density heatmap
    let commit_max = snap
        .hourly_commits
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let commit_spans = build_heatrow_commits(&snap.hourly_commits, commit_max, bar_width);
    let commit_line = Line::from(
        std::iter::once(Span::styled(
            format!(" {:<width$}", "Commits", width = label_width - 1),
            Style::default().fg(Color::White),
        ))
        .chain(commit_spans)
        .collect::<Vec<_>>(),
    );
    frame.render_widget(Paragraph::new(commit_line), rows[1]);

    // Workflow rows
    for (i, wf) in snap.workflows.iter().enumerate() {
        if i + 2 >= rows.len() {
            break;
        }
        let status_spans = build_heatrow_status(&wf.hourly_status, bar_width);
        let wf_line = Line::from(
            std::iter::once(Span::styled(
                format!(" {:<width$}", truncate_str(&wf.short_name, label_width - 1), width = label_width - 1),
                Style::default().fg(Color::White),
            ))
            .chain(status_spans)
            .collect::<Vec<_>>(),
        );
        frame.render_widget(Paragraph::new(wf_line), rows[i + 2]);
    }
}

/// Build colored spans for a commit density heatrow.
/// Maps each hour to a green-intensity glyph: high=bright, low=dark, zero=dark gray.
fn build_heatrow_commits(hourly: &[f64], max_val: f64, width: usize) -> Vec<Span<'static>> {
    let num_slots = hourly.len();
    let mut spans = Vec::with_capacity(width);
    for col in 0..width {
        let slot = if num_slots > 0 {
            col * num_slots / width.max(1)
        } else {
            0
        };
        let val = hourly.get(slot).copied().unwrap_or(0.0);
        let intensity = val / max_val;
        let (ch, color) = if intensity > 0.66 {
            ('\u{2588}', Color::Green) // █ full block
        } else if intensity > 0.33 {
            ('\u{2592}', Color::Green) // ▒ medium shade
        } else if intensity > 0.0 {
            ('\u{2591}', Color::DarkGray) // ░ light shade
        } else {
            ('\u{2591}', Color::DarkGray) // ░ no activity
        };
        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(color),
        ));
    }
    spans
}

/// Build colored spans for a workflow status heatrow.
/// 1.0=green, -1.0=red, 0.5=yellow, 0.25=dark yellow, 0.0=dark gray.
fn build_heatrow_status(hourly: &[f64], width: usize) -> Vec<Span<'static>> {
    let num_slots = hourly.len();
    let mut spans = Vec::with_capacity(width);
    for col in 0..width {
        let slot = if num_slots > 0 {
            col * num_slots / width.max(1)
        } else {
            0
        };
        let val = hourly.get(slot).copied().unwrap_or(0.0);
        let (ch, color) = match () {
            _ if val >= 1.0 => ('\u{2588}', Color::Green),     // █ pass
            _ if val <= -1.0 => ('\u{2588}', Color::Red),      // █ fail
            _ if val >= 0.5 => ('\u{2592}', Color::Yellow),    // ▒ in_progress
            _ if val >= 0.25 => ('\u{2591}', Color::Yellow),   // ░ cancelled
            _ => ('\u{2591}', Color::DarkGray),                 // ░ no run
        };
        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(color),
        ));
    }
    spans
}

/// Truncate a string to fit in `max_len` chars.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 2 {
        format!("{}..", &s[..max_len - 2])
    } else {
        s[..max_len].to_string()
    }
}

/// Compact proc/pod panel for one machine in the bottom row.
fn draw_mini_proc(frame: &mut Frame, app: &App, ms: &MachineState, area: Rect) {
    let title = format!(" {} ", ms.name);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if ms.is_local {
        let filtered: Vec<_> = ms
            .processes
            .iter()
            .filter(|p| match ms.proc_filter {
                ProcFilter::All => true,
                ProcFilter::CurrentUser => p.user == app.current_username,
                ProcFilter::HideSelf => p.user != app.current_username,
            })
            .collect();

        if filtered.is_empty() {
            let msg = Paragraph::new(" waiting...")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, inner);
            return;
        }

        let visible = inner.height as usize;
        let constraints: Vec<Constraint> = (0..visible)
            .map(|_| Constraint::Length(1))
            .collect();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let width = inner.width as usize;
        for (i, row_area) in rows.iter().enumerate() {
            if i >= filtered.len() {
                break;
            }
            let p = filtered[i];
            let cmd_w = width.saturating_sub(18);
            let cmd = if p.command.len() > cmd_w && cmd_w > 3 {
                format!("{}...", &p.command[..cmd_w - 3])
            } else {
                p.command.clone()
            };
            let line = Line::from(vec![
                Span::styled(
                    format!("{:>5.1} ", p.cpu_pct),
                    Style::default().fg(value_color(p.cpu_pct / 100.0)),
                ),
                Span::styled(
                    format!("{:>5.1} ", p.mem_pct),
                    Style::default().fg(value_color(p.mem_pct / 100.0)),
                ),
                Span::styled(cmd, Style::default().fg(Color::DarkGray)),
            ]);
            frame.render_widget(Paragraph::new(line), *row_area);
        }
    } else {
        if ms.pods.is_empty() {
            let msg = Paragraph::new(" waiting...")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, inner);
            return;
        }

        let visible = inner.height as usize;
        let constraints: Vec<Constraint> = (0..visible)
            .map(|_| Constraint::Length(1))
            .collect();
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, row_area) in rows.iter().enumerate() {
            if i >= ms.pods.len() {
                break;
            }
            let pod = &ms.pods[i];
            let ns = if pod.namespace.len() > 8 {
                format!("{}.", &pod.namespace[..7])
            } else {
                format!("{:<8}", pod.namespace)
            };
            let name_w = (inner.width as usize).saturating_sub(10);
            let name = if pod.name.len() > name_w && name_w > 3 {
                format!("{}...", &pod.name[..name_w - 3])
            } else {
                pod.name.clone()
            };
            let line = Line::from(vec![
                Span::styled(format!("{ns} "), Style::default().fg(Color::Cyan)),
                Span::styled(name, Style::default().fg(Color::DarkGray)),
            ]);
            frame.render_widget(Paragraph::new(line), *row_area);
        }
    }
}

/// 1D butterfly network meter.
fn draw_net_butterfly_meter(
    frame: &mut Frame,
    tx_bytes: f64,
    rx_bytes: f64,
    label_width: usize,
    area: Rect,
) {
    let width = area.width as usize;
    if width < 10 {
        return;
    }

    let line = build_net_butterfly_line(tx_bytes, rx_bytes, label_width, width);
    frame.render_widget(Paragraph::new(line), area);
}

fn build_net_butterfly_line(
    tx_bytes: f64,
    rx_bytes: f64,
    label_width: usize,
    width: usize,
) -> Line<'static> {
    let prefix = format!("NET:{:<w$}", format_bytes(tx_bytes), w = label_width);
    let suffix = format!("{:>w$}", format_bytes(rx_bytes), w = label_width);
    let graph_width = width.saturating_sub(prefix.len() + suffix.len());
    if graph_width <= 1 {
        return Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Cyan)),
            Span::styled("|", Style::default().fg(Color::DarkGray)),
            Span::styled(suffix, Style::default().fg(Color::Green)),
        ]);
    }

    let content_width = graph_width - 1;
    let tx_width = content_width.div_ceil(2);
    let rx_width = content_width / 2;
    let max_rate = tx_bytes.max(rx_bytes).max(10_240.0);
    let tx_fill = ((tx_bytes / max_rate) * tx_width as f64).round() as usize;
    let rx_fill = ((rx_bytes / max_rate) * rx_width as f64).round() as usize;

    let tx_blank = tx_width.saturating_sub(tx_fill);
    let tx_bar: String = (0..tx_width)
        .map(|col| {
            if col < tx_blank {
                ' '
            } else {
                let intensity = (col - tx_blank) as f64 / tx_width.max(1) as f64;
                intensity_char(intensity)
            }
        })
        .collect();

    let rx_bar: String = (0..rx_width)
        .map(|col| {
            if col < rx_fill {
                let intensity = 1.0 - (col as f64 / rx_width.max(1) as f64);
                intensity_char(intensity)
            } else {
                ' '
            }
        })
        .collect();

    let tx_color = value_color((tx_bytes / max_rate).min(1.0));
    let rx_color = value_color((rx_bytes / max_rate).min(1.0));

    Line::from(vec![
        Span::styled(prefix, Style::default().fg(Color::Cyan)),
        Span::styled(tx_bar, Style::default().fg(tx_color)),
        Span::styled("|", Style::default().fg(Color::DarkGray)),
        Span::styled(rx_bar, Style::default().fg(rx_color)),
        Span::styled(suffix, Style::default().fg(Color::Green)),
    ])
}

fn intensity_char(intensity: f64) -> char {
    if intensity > 0.66 {
        '\u{2588}'
    } else if intensity > 0.33 {
        '\u{2592}'
    } else if intensity > 0.0 {
        '\u{2591}'
    } else {
        ' '
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_text(line: Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn butterfly_meter_uses_full_width_and_keeps_midline_stable() {
        let label_width = 5;
        let width = 24;
        let prefix = format!("NET:{:<w$}", format_bytes(0.0), w = label_width);
        let suffix = format!("{:>w$}", format_bytes(0.0), w = label_width);
        let graph_width = width - prefix.len() - suffix.len();
        let expected_midline = prefix.len() + (graph_width - 1).div_ceil(2);

        let text = line_text(build_net_butterfly_line(0.0, 0.0, label_width, width));

        assert_eq!(text.len(), width);
        assert_eq!(text.match_indices('|').count(), 1);
        assert_eq!(text.find('|'), Some(expected_midline));
    }
}

/// Red alert bar showing pipeline failures on watched branches.
fn draw_pipeline_alerts(frame: &mut Frame, app: &App, area: Rect) {
    let count = app.pipeline_failures.len();
    let title = format!(" PIPELINE FAILURES ({}) ", count);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Red));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = app
        .pipeline_failures
        .iter()
        .take(inner.height as usize)
        .map(|f| {
            let short_repo = f.repo.rsplit('/').next().unwrap_or(&f.repo);
            Line::from(vec![
                Span::styled(
                    " FAIL ",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{short_repo}"),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {} ", f.workflow_name),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    f.failed_at.clone(),
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}
