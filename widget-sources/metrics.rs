// Copyright 2026 The Gilamonster Authors
// SPDX-License-Identifier: Apache-2.0

//! Metrics tab — btop-style 2x2 grid of machine health cards.
//!
//! # Widget vocabulary (from btop)
//!
//! In this codebase, "graph" and "meter" refer to specific btop-derived
//! widget types that render on limited-Unicode consoles (░▒█ only):
//!
//! - **Graph** (heat graph): Multi-row rolling time series. Each column
//!   is a data point, rows stack vertically. Fill chars ░▒█ with
//!   per-row color gradient (green bottom → red top). Used for CPU
//!   history, memory history, GPU history, network I/O history.
//!
//! - **Meter** (heat meter): Single-row horizontal bar. Each filled █
//!   gets its own color based on position (green left → red right).
//!   Unfilled positions show `·`. Used for storage bars.
//!
//! Both use only 3 fill glyphs (░ U+2591, ▒ U+2592, █ U+2588) verified
//! present in the Uni2-Terminus16 PSF console font. All visual intensity
//! comes from ANSI color, not glyph variety — learned from btop source.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Direction;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, MachineState};
use crate::event::MachineMetrics;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    // Sub-tab bar (1 line) + content (rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(5)])
        .split(area);

    super::sub_tabs::draw(frame, app, chunks[0]);

    if app.metrics_sub_tab_idx == 0 {
        draw_summary(frame, app, chunks[1]);
    } else {
        super::machine_tab::draw(frame, app, chunks[1]);
    }
}

/// gnuc cell — 4 quadrants with local data: CPU cores, MEM+NET, Storage, GPU.
fn draw_gnuc_cell(frame: &mut Frame, app: &App, m: &MachineMetrics, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" gnuc ")
        .border_style(Style::default().fg(machine_health_color(m)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 2x2 quadrants
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[0]);
    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(rows[1]);

    // Top-left: per-core CPU graphs
    draw_cpu_cores(frame, app, top[0]);
    // Top-right: MEM + NET
    draw_mem_net(frame, m, app.effective_net_max_kbps(), top[1]);
    // Bottom-left: storage bars
    draw_storage_bars(frame, app, bot[0]);
    // Bottom-right: GPU
    draw_gpu_inset(frame, m, bot[1]);
}

fn draw_cpu_cores(frame: &mut Frame, app: &App, area: Rect) {
    let ms = match app.machine_states.first() {
        Some(ms) => ms,
        None => return,
    };
    let cores = &ms.cpu_per_core;
    let histories = &ms.cpu_core_histories;
    if cores.is_empty() {
        return;
    }

    // One line per core: "C00 [heatgraph] 45%"
    let constraints: Vec<Constraint> = cores.iter().map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, &usage) in cores.iter().enumerate() {
        if i >= rows.len() {
            break;
        }
        let current_color = value_color(usage / 100.0);
        let history = histories.get(i).cloned().unwrap_or_default();
        let data: Vec<u64> = history.iter().map(|v| *v as u64).collect();

        let bar_width = area.width.saturating_sub(8) as usize;
        let blocks: &[char] = &[' ', '░', '▒', '█'];

        // Build per-character spans — each char gets its own color
        let mut spans: Vec<Span> = Vec::with_capacity(bar_width + 4);
        spans.push(Span::styled(
            format!("{i:>2} "),
            Style::default().fg(Color::DarkGray),
        ));

        if !data.is_empty() {
            let points: Vec<&u64> = data.iter().rev().take(bar_width).collect::<Vec<_>>();
            // Pad left
            for _ in 0..(bar_width.saturating_sub(points.len())) {
                spans.push(Span::raw(" "));
            }
            for &&v in points.iter().rev() {
                let idx = ((v as usize) * 3 / 100).min(3);
                let ch = blocks[idx];
                let ch_color = value_color(v as f64 / 100.0);
                if ch == ' ' {
                    spans.push(Span::raw(" "));
                } else {
                    spans.push(Span::styled(
                        String::from(ch),
                        Style::default().fg(ch_color),
                    ));
                }
            }
        } else {
            spans.push(Span::raw(" ".repeat(bar_width)));
        }

        spans.push(Span::styled(
            format!(" {usage:>3.0}%"),
            Style::default().fg(current_color),
        ));

        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line), rows[i]);
    }
}

fn draw_mem_net(frame: &mut Frame, m: &MachineMetrics, net_max_kbps: u64, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // MEM label
            Constraint::Length(2), // MEM graph
            Constraint::Length(1), // NET label
            Constraint::Min(2),    // NET graph
        ])
        .split(area);

    let mem_line = Line::from(vec![
        Span::styled(
            "MEM ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:.1}/{:.0}G", m.memory_used_gb, m.memory_total_gb),
            Style::default().fg(value_color(m.memory_percent / 100.0)),
        ),
    ]);
    frame.render_widget(Paragraph::new(mem_line), rows[0]);
    draw_graph(frame, &m.mem_history, 100, Color::Blue, rows[1]);

    let net_line = Line::from(vec![
        Span::styled("NET ", Style::default().fg(Color::DarkGray)),
        Span::styled("↓", Style::default().fg(Color::Green)),
        Span::raw(format_bytes(m.net_rx_bytes_sec)),
        Span::raw(" "),
        Span::styled("↑", Style::default().fg(Color::Red)),
        Span::raw(format_bytes(m.net_tx_bytes_sec)),
    ]);
    frame.render_widget(Paragraph::new(net_line), rows[2]);
    draw_graph(frame, &m.net_rx_history, net_max_kbps, Color::Green, rows[3]);
}

fn draw_storage_bars(frame: &mut Frame, app: &App, area: Rect) {
    let disks = match app.machine_states.first() {
        Some(ms) => &ms.disks,
        None => return,
    };
    if disks.is_empty() {
        let msg = Paragraph::new("no disk data").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    let constraints: Vec<Constraint> = disks.iter().map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, disk) in disks.iter().enumerate() {
        if i >= rows.len() {
            break;
        }
        let pct = if disk.total_gb > 0.0 {
            (disk.used_gb / disk.total_gb) * 100.0
        } else {
            0.0
        };
        let label = if disk.mount_point == "/" {
            "/".to_string()
        } else {
            disk.mount_point
                .rsplit('/')
                .next()
                .unwrap_or(&disk.mount_point)
                .to_string()
        };
        let label_short = if label.len() > 4 {
            format!("{}.", &label[..3])
        } else {
            format!("{label:<4}")
        };
        let right = format!("{:.0}/{:.0}G", disk.used_gb, disk.total_gb);
        draw_heat_meter(frame, &label_short, pct, &right, rows[i]);
    }
}

/// btop-style heat meter — single row, each filled █ gets its own
/// gradient color (green→yellow→red left to right).
pub fn draw_heat_meter(frame: &mut Frame, label: &str, percent: f64, right_label: &str, area: Rect) {
    let right_len = right_label.len() + 1;
    let bar_width = area.width.saturating_sub(5 + right_len as u16) as usize;
    let filled = ((bar_width as f64) * (percent / 100.0).clamp(0.0, 1.0)) as usize;

    let mut spans: Vec<Span> = Vec::with_capacity(bar_width + 6);

    spans.push(Span::styled(
        format!("{label:<4} "),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));

    // Each filled position gets a color based on its position in the bar
    for pos in 0..bar_width {
        if pos < filled {
            let pos_pct = (pos as f64) / (bar_width as f64);
            let color = if pos_pct >= 0.8 {
                Color::Red
            } else if pos_pct >= 0.6 {
                Color::LightRed
            } else if pos_pct >= 0.4 {
                Color::Yellow
            } else if pos_pct >= 0.2 {
                Color::Green
            } else {
                Color::DarkGray
            };
            spans.push(Span::styled("█", Style::default().fg(color)));
        } else {
            spans.push(Span::styled("·", Style::default().fg(Color::DarkGray)));
        }
    }

    spans.push(Span::raw(format!(" {right_label}")));

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

/// Remote machine cell — same 4-quadrant layout as gnuc, but no GPU.
/// Data comes from Prometheus (via MachineMetrics).
fn draw_machine_cell(frame: &mut Frame, m: &MachineMetrics, net_max_kbps: u64, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", m.name))
        .border_style(Style::default().fg(machine_health_color(m)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 2x2 quadrants (same as gnuc, minus GPU)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);

    let bot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);

    // Top-left: CPU graph
    draw_cpu_graph(frame, m, top[0]);
    // Top-right: MEM graph
    draw_mem_graph(frame, m, top[1]);
    // Bottom-left: storage meter
    draw_disk_meter(frame, m, bot[0]);
    // Bottom-right: NET graph
    draw_net_graph(frame, m, net_max_kbps, bot[1]);
}

fn draw_cpu_graph(frame: &mut Frame, m: &MachineMetrics, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(2)])
        .split(area);

    let cpu_line = Line::from(vec![
        Span::styled(
            "CPU ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:.0}%", m.cpu_percent),
            Style::default().fg(value_color(m.cpu_percent / 100.0)),
        ),
    ]);
    frame.render_widget(Paragraph::new(cpu_line), rows[0]);
    draw_graph(frame, &m.cpu_history, 100, Color::Cyan, rows[1]);
}

fn draw_mem_graph(frame: &mut Frame, m: &MachineMetrics, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(2)])
        .split(area);

    let mem_line = Line::from(vec![
        Span::styled(
            "MEM ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:.1}/{:.0}G", m.memory_used_gb, m.memory_total_gb),
            Style::default().fg(value_color(m.memory_percent / 100.0)),
        ),
    ]);
    frame.render_widget(Paragraph::new(mem_line), rows[0]);
    draw_graph(frame, &m.mem_history, 100, Color::Blue, rows[1]);
}

fn draw_disk_meter(frame: &mut Frame, m: &MachineMetrics, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let disk_used = m.disk_total_gb - m.disk_free_gb;
    let disk_pct = if m.disk_total_gb > 0.0 {
        (disk_used / m.disk_total_gb) * 100.0
    } else {
        0.0
    };
    let disk_label = format!("{:.0}/{:.0}G", disk_used, m.disk_total_gb);
    draw_heat_meter(frame, "DSK", disk_pct, &disk_label, rows[0]);
}

fn draw_net_graph(frame: &mut Frame, m: &MachineMetrics, net_max_kbps: u64, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(2)])
        .split(area);

    let net_line = Line::from(vec![
        Span::styled("NET ", Style::default().fg(Color::DarkGray)),
        Span::styled("↓", Style::default().fg(Color::Green)),
        Span::raw(format_bytes(m.net_rx_bytes_sec)),
        Span::raw(" "),
        Span::styled("↑", Style::default().fg(Color::Red)),
        Span::raw(format_bytes(m.net_tx_bytes_sec)),
    ]);
    frame.render_widget(Paragraph::new(net_line), rows[0]);
    draw_graph(frame, &m.net_rx_history, net_max_kbps, Color::Green, rows[1]);
}

/// Small purple GPU inset box inside a machine cell.
fn draw_gpu_inset(frame: &mut Frame, m: &MachineMetrics, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" GPU ")
        .border_style(Style::default().fg(Color::Magenta));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // GPU label
            Constraint::Length(2), // GPU graph
            Constraint::Length(1), // VRAM label
            Constraint::Length(2), // VRAM graph
            Constraint::Length(1), // Temp label
            Constraint::Min(2),   // Temp graph
        ])
        .split(inner);

    if let Some(gpu) = m.gpu_percent {
        let gpu_line = Line::from(vec![
            Span::styled("UTL ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{gpu:.0}%"),
                Style::default()
                    .fg(value_color(gpu / 100.0))
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(gpu_line), rows[0]);
    }
    draw_graph(frame, &m.gpu_history, 100, Color::Magenta, rows[1]);

    if let Some((used, total)) = m.gpu_memory_used_mb.zip(m.gpu_memory_total_mb) {
        let vram_line = Line::from(vec![
            Span::styled("MEM ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.0}/{:.0}M", used, total),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(vram_line), rows[2]);
    }
    draw_graph(frame, &m.gpu_mem_history, 100, Color::LightMagenta, rows[3]);

    // Temp label + graph
    if let Some(temp) = m.gpu_temp_c {
        let temp_color = if temp >= 80.0 {
            Color::Red
        } else if temp >= 60.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        let temp_line = Line::from(vec![
            Span::styled("TMP ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{temp:.0}°C"),
                Style::default().fg(temp_color),
            ),
        ]);
        frame.render_widget(Paragraph::new(temp_line), rows[4]);
    }
    draw_graph(frame, &m.gpu_temp_history, 100, Color::Red, rows[5]);
}

/// btop-style HeatGraph using ░▒█ with color gradients per row.
///
/// The trick from btop: the glyph only gives 3 fill levels, but
/// the COLOR changes per row (green at bottom → yellow → red at top)
/// to show intensity. Visual effect comes from color, not glyph variety.
///
/// Console-verified chars: space, ░ (U+2591), ▒ (U+2592), █ (U+2588).
pub fn draw_graph(frame: &mut Frame, history: &[f64], max: u64, _color: Color, area: Rect) {
    if history.is_empty() || area.height == 0 {
        return;
    }

    let width = area.width as usize;
    let height = area.height as usize;
    let max_val = if max > 0 {
        max as f64
    } else {
        history.iter().cloned().fold(1.0_f64, f64::max).max(1.0)
    };

    let data: Vec<f64> = history.iter().rev().take(width).rev().copied().collect();

    for row in 0..height {
        let row_area = Rect {
            x: area.x,
            y: area.y + row as u16,
            width: area.width,
            height: 1,
        };

        let row_bottom = (height - 1 - row) as f64 / height as f64 * max_val;
        let row_top = (height - row) as f64 / height as f64 * max_val;
        let row_mid = (row_bottom + row_top) / 2.0;

        let mut spans: Vec<Span> = Vec::with_capacity(width);

        // Pad left
        for _ in 0..(width.saturating_sub(data.len())) {
            spans.push(Span::raw(" "));
        }

        // Each filled character gets color based on its DATA VALUE,
        // not the row position. 30% memory = green, 90% = red.
        for &val in &data {
            let val_color = value_color(val / max_val);
            if val >= row_top {
                spans.push(Span::styled("█", Style::default().fg(val_color)));
            } else if val >= row_mid {
                spans.push(Span::styled("▒", Style::default().fg(val_color)));
            } else if val > row_bottom {
                spans.push(Span::styled("░", Style::default().fg(val_color)));
            } else {
                spans.push(Span::raw(" "));
            }
        }

        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line), row_area);
    }
}

/// Inverted HeatGraph — grows UP from the bottom (for butterfly graph RX half).
/// Same as draw_graph but row order is flipped: row 0 = bottom, last row = top.
pub fn draw_graph_inverted(
    frame: &mut Frame,
    history: &[f64],
    max: u64,
    _color: Color,
    area: Rect,
) {
    if history.is_empty() || area.height == 0 {
        return;
    }

    let width = area.width as usize;
    let height = area.height as usize;
    let max_val = if max > 0 {
        max as f64
    } else {
        history.iter().cloned().fold(1.0_f64, f64::max).max(1.0)
    };

    let data: Vec<f64> = history.iter().rev().take(width).rev().copied().collect();

    // Inverted: row 0 is the BOTTOM of the graph (lowest thresholds),
    // last row is the TOP (highest thresholds). Bars grow upward.
    for row in 0..height {
        let row_area = Rect {
            x: area.x,
            y: area.y + row as u16,
            width: area.width,
            height: 1,
        };

        // Invert: this row represents high values at the top, low at bottom
        let row_bottom = row as f64 / height as f64 * max_val;
        let row_top = (row + 1) as f64 / height as f64 * max_val;
        let row_mid = (row_bottom + row_top) / 2.0;

        let mut spans: Vec<Span> = Vec::with_capacity(width);

        for _ in 0..(width.saturating_sub(data.len())) {
            spans.push(Span::raw(" "));
        }

        for &val in &data {
            let val_color = value_color(val / max_val);
            if val >= row_top {
                spans.push(Span::styled("█", Style::default().fg(val_color)));
            } else if val >= row_mid {
                spans.push(Span::styled("▒", Style::default().fg(val_color)));
            } else if val > row_bottom {
                spans.push(Span::styled("░", Style::default().fg(val_color)));
            } else {
                spans.push(Span::raw(" "));
            }
        }

        let line = Line::from(spans);
        frame.render_widget(Paragraph::new(line), row_area);
    }
}

fn draw_empty_cell(frame: &mut Frame, _name: &str, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(block, area);
}

fn draw_bar_line(frame: &mut Frame, label: &str, value: f64, max: f64, unit: &str, area: Rect) {
    let ratio = (value / max).clamp(0.0, 1.0);
    let color = value_color(ratio);
    let bar_width = area.width.saturating_sub(12) as usize;
    let filled = (bar_width as f64 * ratio) as usize;
    let empty = bar_width.saturating_sub(filled);

    let line = Line::from(vec![
        Span::styled(
            format!("{label:<3} "),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled("·".repeat(empty), Style::default().fg(Color::DarkGray)),
        Span::raw(format!(" {value:.0}{unit}")),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_bar_with_label(
    frame: &mut Frame,
    label: &str,
    percent: f64,
    right_label: &str,
    area: Rect,
) {
    let ratio = (percent / 100.0).clamp(0.0, 1.0);
    let color = value_color(ratio);
    let right_len = right_label.len() + 1;
    let bar_width = area.width.saturating_sub(5 + right_len as u16) as usize;
    let filled = (bar_width as f64 * ratio) as usize;
    let empty = bar_width.saturating_sub(filled);

    let line = Line::from(vec![
        Span::styled(
            format!("{label:<3} "),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled("·".repeat(empty), Style::default().fg(Color::DarkGray)),
        Span::raw(format!(" {right_label}")),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

pub fn value_color(ratio: f64) -> Color {
    if ratio >= 0.9 {
        Color::Red
    } else if ratio >= 0.7 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub fn machine_health_color(m: &MachineMetrics) -> Color {
    let worst = m.cpu_percent.max(m.memory_percent);
    if worst >= 90.0 {
        Color::Red
    } else if worst >= 70.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub fn format_bytes(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_073_741_824.0 {
        format!("{:.1}G/s", bytes_per_sec / 1_073_741_824.0)
    } else if bytes_per_sec >= 1024.0 * 1024.0 {
        format!("{:.1}M/s", bytes_per_sec / (1024.0 * 1024.0))
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.0}K/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0}B/s", bytes_per_sec)
    }
}

/// Summary sub-tab: all machines as rows in a compact table.
fn draw_summary(frame: &mut Frame, app: &App, area: Rect) {
    if area.height < 2 || area.width < 20 {
        return;
    }

    // Top status line: playback volume + fullest disk, always visible so a
    // full disk shows on the main display (not only spoken).
    draw_summary_status(frame, app, Rect { height: 1, ..area });
    let area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    // Reserve 1 column on the right for the scrollbar
    let content_area = Rect {
        width: area.width.saturating_sub(1),
        ..area
    };
    let scrollbar_area = Rect {
        x: area.x + content_area.width,
        width: 1,
        ..area
    };

    let machine_count = app.metrics.machines.len();

    match summary_layout(content_area.height, machine_count, app.summary_graph_rows as u16) {
        SummaryLayout::Expanded { graph_rows } => {
            draw_summary_expanded(frame, app, content_area, graph_rows)
        }
        SummaryLayout::Compact => draw_summary_compact(frame, app, content_area),
    }

    draw_scrollbar(frame, scrollbar_area, machine_count, 0);
}

/// One-line status header for the summary tab: playback volume and the
/// fullest disk. At/above the configured warn threshold the disk turns red
/// so a filling disk is visible on the main display, not only spoken.
/// ASCII-only — Monty runs on the Linux console (limited font, no emoji).
fn draw_summary_status(frame: &mut Frame, app: &App, area: Rect) {
    let warn = app.attention_config.disk_warn_pct;
    let caution = (warn - 10.0).max(0.0);

    let vol = app
        .volume_percent
        .map(|v| format!("{v}%"))
        .unwrap_or_else(|| "--".into());

    let mut spans = vec![
        Span::styled("Vol ", Style::default().fg(Color::Cyan)),
        Span::styled(vol, Style::default().add_modifier(Modifier::BOLD)),
    ];

    let worst = app
        .metrics
        .machines
        .iter()
        .filter(|m| m.disk_total_gb > 0.0)
        .map(|m| {
            (
                m.name.as_str(),
                (m.disk_total_gb - m.disk_free_gb) / m.disk_total_gb * 100.0,
            )
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    if let Some((name, pct)) = worst {
        let (label, color, bold) = if pct >= warn {
            (format!("   DISK FULL: {name} {pct:.0}%"), Color::Red, true)
        } else if pct >= caution {
            (format!("   disk {name} {pct:.0}%"), Color::Yellow, false)
        } else {
            (format!("   disk {name} {pct:.0}%"), Color::DarkGray, false)
        };
        let mut style = Style::default().fg(color);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        spans.push(Span::styled(label, style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Pure layout decision for the summary tab — picks expanded vs compact and
/// the graph row count, given the rendering budget. Lives outside `draw_*`
/// so it's testable; rendering then trusts whatever this returns.
///
/// Below MIN_GRAPH_ROWS the graphs lose enough resolution that compact (a
/// 2-line meter row per machine) is the better experience. Above the
/// configured ceiling we don't grow rows — a 2-machine view should look
/// the same before and after this fits-the-screen logic landed.
#[derive(Debug, PartialEq, Eq)]
enum SummaryLayout {
    Expanded { graph_rows: u16 },
    Compact,
}

fn summary_layout(area_height: u16, machine_count: usize, configured_graph_rows: u16) -> SummaryLayout {
    const MIN_GRAPH_ROWS: u16 = 3;
    if machine_count == 0 {
        return SummaryLayout::Compact;
    }
    let n = machine_count as u16;
    let per_machine_max = area_height.saturating_sub(1) / n;
    let fit_graph_rows = per_machine_max.saturating_sub(2);
    let graph_rows = configured_graph_rows.min(fit_graph_rows);
    if graph_rows >= MIN_GRAPH_ROWS {
        SummaryLayout::Expanded { graph_rows }
    } else {
        SummaryLayout::Compact
    }
}

/// Expanded summary: each machine gets a bordered block with HeatGraphs + proc/pod detail.
fn draw_summary_expanded(frame: &mut Frame, app: &App, area: Rect, graph_rows: u16) {
    // Each machine: bordered block (2 border rows + graph_rows content)
    let block_height = graph_rows + 2; // top border + content + bottom border
    let mut y = area.y;

    for (i, m) in app.metrics.machines.iter().enumerate() {
        let ms = app.machine_states.get(i);
        if y + block_height > area.y + area.height {
            break;
        }

        // Machine block with title
        let connected = ms.map(|s| s.connected).unwrap_or(false);
        let border_color = if connected { Color::Green } else { Color::DarkGray };

        let is_local = ms.map(|s| s.is_local).unwrap_or(false);
        let has_gpu = ms.map(|s| s.has_gpu).unwrap_or(false);

        // Build title with key stats
        let cpu_str = format!("CPU:{:.0}%", m.cpu_percent);
        let mem_str = format!("MEM:{:.0}%", m.memory_percent);
        let net_str = format!("TX:{} RX:{}", format_bytes(m.net_tx_bytes_sec), format_bytes(m.net_rx_bytes_sec));
        let gpu_str = if has_gpu {
            format!("GPU:{:.0}%", m.gpu_percent.unwrap_or(0.0))
        } else {
            String::new()
        };
        let title = if gpu_str.is_empty() {
            format!(" {} | {} | {} | {} ", m.name, cpu_str, mem_str, net_str)
        } else {
            format!(" {} | {} | {} | {} | {} ", m.name, cpu_str, mem_str, net_str, gpu_str)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color));

        let block_area = Rect { x: area.x, y, width: area.width, height: block_height };
        let inner = block.inner(block_area);
        frame.render_widget(block, block_area);

        // Divide inner into columns: CPU | MEM | NET | GPU | PROC
        let num_cols = if has_gpu { 5 } else { 4 };
        let col_w = inner.width / num_cols as u16;
        let last_col_w = inner.width - col_w * (num_cols - 1) as u16;

        let mut col_x = inner.x;

        // Helper: render a titled cell with a border and graph inside
        let render_cell = |frame: &mut Frame, title: &str, cell_color: Color, x: u16, w: u16| -> Rect {
            let cell = Rect { x, y: inner.y, width: w, height: inner.height };
            let cell_block = Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", title))
                .border_style(Style::default().fg(cell_color));
            let cell_inner = cell_block.inner(cell);
            frame.render_widget(cell_block, cell);
            cell_inner
        };

        // CPU cell
        let cpu_temp = m.cpu_temp_c.map(|t| format!(" {:.0}C", t)).unwrap_or_default();
        let cpu_inner = render_cell(frame, &format!("CPU {:.0}%{}", m.cpu_percent, cpu_temp), Color::Cyan, col_x, col_w);
        draw_graph(frame, &m.cpu_history, 100, Color::Cyan, cpu_inner);
        col_x += col_w;

        // MEM cell
        let mem_inner = render_cell(frame, &format!("MEM {:.0}% {:.0}/{:.0}G", m.memory_percent, m.memory_used_gb, m.memory_total_gb), Color::Yellow, col_x, col_w);
        draw_graph(frame, &m.mem_history, 100, Color::Yellow, mem_inner);
        col_x += col_w;

        // NET cell (butterfly)
        let net_inner = render_cell(frame, &format!("NET TX:{} RX:{}", format_bytes(m.net_tx_bytes_sec), format_bytes(m.net_rx_bytes_sec)), Color::Green, col_x, col_w);
        if net_inner.height >= 2 {
            let net_half = net_inner.height / 2;
            let net_tx = Rect { height: net_half, ..net_inner };
            let net_rx = Rect { y: net_inner.y + net_half, height: net_inner.height - net_half, ..net_inner };
            let net_max_kbps = app.effective_net_max_kbps();
            draw_graph(frame, &m.net_tx_history, net_max_kbps, Color::Cyan, net_tx);
            draw_graph_inverted(frame, &m.net_rx_history, net_max_kbps, Color::Green, net_rx);
        }
        col_x += col_w;

        // GPU cell (butterfly, if present)
        if has_gpu {
            let gpu_pct = m.gpu_percent.unwrap_or(0.0);
            let gpu_temp = m.gpu_temp_c.map(|t| format!(" {:.0}C", t)).unwrap_or_default();
            let gpu_inner = render_cell(frame, &format!("GPU {:.0}%{}", gpu_pct, gpu_temp), Color::Magenta, col_x, col_w);
            if gpu_inner.height >= 2 {
                let gpu_half = gpu_inner.height / 2;
                let gpu_util = Rect { height: gpu_half, ..gpu_inner };
                let gpu_mem = Rect { y: gpu_inner.y + gpu_half, height: gpu_inner.height - gpu_half, ..gpu_inner };
                draw_graph(frame, &m.gpu_history, 100, Color::Magenta, gpu_util);
                draw_graph_inverted(frame, &m.gpu_mem_history, 100, Color::LightMagenta, gpu_mem);
            }
            col_x += col_w;
        }

        // Proc/Pod cell (last column, remaining width)
        let proc_w = area.x + area.width - col_x - 1;
        let proc_title = if is_local { "PROC" } else { "POD" };
        let proc_count = if is_local {
            ms.map(|s| s.processes.len()).unwrap_or(0)
        } else {
            ms.map(|s| s.pods.len()).unwrap_or(0)
        };
        let proc_cell = Rect { x: col_x, y: inner.y, width: proc_w, height: inner.height };
        let proc_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ({}) ", proc_title, proc_count))
            .border_style(Style::default().fg(Color::DarkGray));
        let proc_area = proc_block.inner(proc_cell);
        frame.render_widget(proc_block, proc_cell);

        if is_local {
            // Show top processes by CPU
            let procs = ms.map(|s| &s.processes[..]).unwrap_or(&[]);
            let visible = inner.height as usize;
            for (j, row_area) in (0..visible).enumerate() {
                if j >= procs.len() { break; }
                let p = &procs[j];
                let cmd_w = proc_area.width as usize;
                let cmd = if cmd_w > 12 {
                    let cw = cmd_w - 12;
                    let c = if p.command.len() > cw { format!("{}..", &p.command[..cw.saturating_sub(2)]) } else { p.command.clone() };
                    format!("{:>5.1} {:>5.1} {}", p.cpu_pct, p.mem_pct, c)
                } else {
                    format!("{:>5.1}", p.cpu_pct)
                };
                let line = Line::from(Span::styled(
                    cmd,
                    Style::default().fg(value_color(p.cpu_pct / 100.0)),
                ));
                let r = Rect { x: proc_area.x, y: proc_area.y + j as u16, width: proc_area.width, height: 1 };
                frame.render_widget(Paragraph::new(line), r);
            }
        } else {
            // Show pods
            let pods = ms.map(|s| &s.pods[..]).unwrap_or(&[]);
            let visible = inner.height as usize;
            for (j, _) in (0..visible).enumerate() {
                if j >= pods.len() { break; }
                let pod = &pods[j];
                let w = proc_area.width as usize;
                let ns = if pod.namespace.len() > 8 { format!("{}..", &pod.namespace[..6]) } else { pod.namespace.clone() };
                let name_w = w.saturating_sub(10);
                let name = if pod.name.len() > name_w { format!("{}..", &pod.name[..name_w.saturating_sub(2)]) } else { pod.name.clone() };
                let text = format!("{:<8} {}", ns, name);
                let line = Line::from(Span::styled(text, Style::default().fg(Color::DarkGray)));
                let r = Rect { x: proc_area.x, y: proc_area.y + j as u16, width: proc_area.width, height: 1 };
                frame.render_widget(Paragraph::new(line), r);
            }
        }

        y += block_height;
    }
}

/// Compact summary: 2-line rows with label + meters (fallback for many machines).
fn draw_summary_compact(frame: &mut Frame, app: &App, area: Rect) {
    let header_area = Rect { height: 1, ..area };
    let header = build_summary_header(area.width);
    frame.render_widget(Paragraph::new(header), header_area);

    let row_height: u16 = 2;
    let rows_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    for (i, m) in app.metrics.machines.iter().enumerate() {
        let ms = app.machine_states.get(i);
        let row_y = rows_area.y + (i as u16) * row_height;
        if row_y + row_height > rows_area.y + rows_area.height {
            break;
        }

        let label_area = Rect { x: rows_area.x, y: row_y, width: rows_area.width, height: 1 };
        let label = build_summary_label_line(m, ms, area.width);
        frame.render_widget(Paragraph::new(label), label_area);

        let meter_area = Rect { x: rows_area.x, y: row_y + 1, width: rows_area.width, height: 1 };
        draw_summary_meter_line(frame, m, ms, meter_area);
    }
}

/// Column layout helper: returns (name_w, cpu_w, mem_w, net_w, gpu_w, proc_w).
fn summary_col_widths(total_width: u16) -> (u16, u16, u16, u16, u16, u16) {
    let name_w: u16 = 14;
    let remaining = total_width.saturating_sub(name_w);
    let col = remaining / 5;
    let last = remaining - col * 4; // give remainder to last column
    (name_w, col, col, col, col, last)
}

/// Build the header line for the summary table.
fn build_summary_header(width: u16) -> Line<'static> {
    let (name_w, cpu_w, mem_w, net_w, gpu_w, proc_w) = summary_col_widths(width);
    let hdr_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    Line::from(vec![
        Span::styled(format!("{:<w$}", "Machine", w = name_w as usize), hdr_style),
        Span::styled(format!("{:<w$}", "CPU", w = cpu_w as usize), hdr_style),
        Span::styled(format!("{:<w$}", "Mem/Storage", w = mem_w as usize), hdr_style),
        Span::styled(format!("{:<w$}", "Net", w = net_w as usize), hdr_style),
        Span::styled(format!("{:<w$}", "GPU", w = gpu_w as usize), hdr_style),
        Span::styled(format!("{:<w$}", "Proc/Pod", w = proc_w as usize), hdr_style),
    ])
}

/// Build the label line (line 1) for a single machine row.
fn build_summary_label_line<'a>(
    m: &MachineMetrics,
    ms: Option<&MachineState>,
    width: u16,
) -> Line<'a> {
    let (name_w, cpu_w, mem_w, net_w, gpu_w, proc_w) = summary_col_widths(width);

    // Machine name
    let connected = ms.map(|s| s.connected).unwrap_or(false);
    let name_style = if connected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let name_str = format!("{:<w$}", m.name, w = name_w as usize);

    // CPU: percent + temp
    let cpu_ratio = m.cpu_percent / 100.0;
    let cpu_temp_str = m
        .cpu_temp_c
        .map(|t| format!(" {:.0}C", t))
        .unwrap_or_default();
    let cpu_label = format!("{:.0}%{}", m.cpu_percent, cpu_temp_str);
    let cpu_str = format!("{:<w$}", cpu_label, w = cpu_w as usize);

    // MEM: percent + used/total
    let mem_ratio = m.memory_percent / 100.0;
    let mem_label = format!(
        "{:.0}% {:.0}/{:.0}G",
        m.memory_percent, m.memory_used_gb, m.memory_total_gb
    );
    let mem_str = format!("{:<w$}", mem_label, w = mem_w as usize);

    // NET: TX + RX
    let tx_str = format_bytes(m.net_tx_bytes_sec);
    let rx_str = format_bytes(m.net_rx_bytes_sec);

    // GPU: percent + temp (or n/a)
    let has_gpu = ms.map(|s| s.has_gpu).unwrap_or(false);

    // Proc/Pod count
    let is_local = ms.map(|s| s.is_local).unwrap_or(false);
    let proc_label = if is_local {
        let count = ms.map(|s| s.processes.len()).unwrap_or(0);
        format!("{} procs", count)
    } else {
        let count = ms.map(|s| s.pods.len()).unwrap_or(0);
        format!("{} pods", count)
    };

    // Build spans
    let mut spans: Vec<Span> = vec![
        Span::styled(name_str, name_style),
        Span::styled(cpu_str, Style::default().fg(value_color(cpu_ratio))),
        Span::styled(mem_str, Style::default().fg(value_color(mem_ratio))),
    ];

    // NET column: TX: val RX: val
    let net_intensity_tx = (m.net_tx_bytes_sec / 1_048_576.0).clamp(0.0, 1.0);
    let net_intensity_rx = (m.net_rx_bytes_sec / 1_048_576.0).clamp(0.0, 1.0);
    let net_combined = format!("TX:{} RX:{}", tx_str, rx_str);
    let net_padded = format!("{:<w$}", net_combined, w = net_w as usize);
    // Use a single span with blended color based on max intensity
    let net_max = net_intensity_tx.max(net_intensity_rx);
    spans.push(Span::styled(
        net_padded,
        Style::default().fg(value_color(net_max)),
    ));

    // GPU column
    if has_gpu {
        let gpu_pct = m.gpu_percent.unwrap_or(0.0);
        let gpu_temp_str = m
            .gpu_temp_c
            .map(|t| format!(" {:.0}C", t))
            .unwrap_or_default();
        let gpu_label = format!("{:.0}%{}", gpu_pct, gpu_temp_str);
        let gpu_str = format!("{:<w$}", gpu_label, w = gpu_w as usize);
        let gpu_ratio = gpu_pct / 100.0;
        spans.push(Span::styled(
            gpu_str,
            Style::default().fg(value_color(gpu_ratio)),
        ));
    } else {
        let gpu_str = format!("{:<w$}", "n/a", w = gpu_w as usize);
        spans.push(Span::styled(
            gpu_str,
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Proc/Pod column
    let proc_str = format!("{:<w$}", proc_label, w = proc_w as usize);
    spans.push(Span::styled(
        proc_str,
        Style::default().fg(Color::DarkGray),
    ));

    Line::from(spans)
}

/// Draw the meter line (line 2) for a single machine row.
fn draw_summary_meter_line(
    frame: &mut Frame,
    m: &MachineMetrics,
    ms: Option<&MachineState>,
    area: Rect,
) {
    let (name_w, cpu_w, mem_w, net_w, gpu_w, proc_w) = summary_col_widths(area.width);
    let has_gpu = ms.map(|s| s.has_gpu).unwrap_or(false);

    // Name column: blank spacer
    let name_area = Rect {
        x: area.x,
        width: name_w,
        ..area
    };
    frame.render_widget(Paragraph::new(""), name_area);

    // CPU meter
    let cpu_area = Rect {
        x: area.x + name_w,
        width: cpu_w,
        ..area
    };
    draw_heat_meter(frame, "", m.cpu_percent, "", cpu_area);

    // MEM meter
    let mem_area = Rect {
        x: area.x + name_w + cpu_w,
        width: mem_w,
        ..area
    };
    draw_heat_meter(frame, "", m.memory_percent, "", mem_area);

    // NET: compact TX/RX spans
    let net_area = Rect {
        x: area.x + name_w + cpu_w + mem_w,
        width: net_w,
        ..area
    };
    let tx_str = format_bytes(m.net_tx_bytes_sec);
    let rx_str = format_bytes(m.net_rx_bytes_sec);
    let tx_intensity = (m.net_tx_bytes_sec / 1_048_576.0).clamp(0.0, 1.0);
    let rx_intensity = (m.net_rx_bytes_sec / 1_048_576.0).clamp(0.0, 1.0);
    let net_line = Line::from(vec![
        Span::styled("TX:", Style::default().fg(Color::Cyan)),
        Span::styled(tx_str, Style::default().fg(value_color(tx_intensity))),
        Span::styled(" RX:", Style::default().fg(Color::Green)),
        Span::styled(rx_str, Style::default().fg(value_color(rx_intensity))),
    ]);
    frame.render_widget(Paragraph::new(net_line), net_area);

    // GPU meter or n/a
    let gpu_area = Rect {
        x: area.x + name_w + cpu_w + mem_w + net_w,
        width: gpu_w,
        ..area
    };
    if has_gpu {
        let gpu_pct = m.gpu_percent.unwrap_or(0.0);
        draw_heat_meter(frame, "", gpu_pct, "", gpu_area);
    } else {
        frame.render_widget(
            Paragraph::new(Span::styled("n/a", Style::default().fg(Color::DarkGray))),
            gpu_area,
        );
    }

    // Proc/Pod count (text only on meter line)
    let proc_area = Rect {
        x: area.x + name_w + cpu_w + mem_w + net_w + gpu_w,
        width: proc_w,
        ..area
    };
    let is_local = ms.map(|s| s.is_local).unwrap_or(false);
    let count_label = if is_local {
        let count = ms.map(|s| s.processes.len()).unwrap_or(0);
        format!("{} procs", count)
    } else {
        let count = ms.map(|s| s.pods.len()).unwrap_or(0);
        format!("{} pods", count)
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            count_label,
            Style::default().fg(Color::DarkGray),
        )),
        proc_area,
    );
}

/// Draw a simple text-based scrollbar in the given 1-column area.
fn draw_scrollbar(frame: &mut Frame, area: Rect, total_items: usize, scroll_offset: usize) {
    if area.height < 3 {
        return;
    }
    let height = area.height as usize;
    let track_style = Style::default().fg(Color::DarkGray);

    // Top arrow
    frame.render_widget(
        Paragraph::new(Span::styled("^", track_style)),
        Rect {
            x: area.x,
            y: area.y,
            width: 1,
            height: 1,
        },
    );

    // Bottom arrow
    frame.render_widget(
        Paragraph::new(Span::styled("v", track_style)),
        Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: 1,
            height: 1,
        },
    );

    // Track (between arrows)
    let track_height = height.saturating_sub(2);
    if track_height == 0 {
        return;
    }

    // Thumb position: proportional to scroll_offset within total_items
    let thumb_pos = if total_items <= 1 {
        0
    } else {
        (scroll_offset * (track_height.saturating_sub(1))) / total_items.saturating_sub(1)
    };

    for row in 0..track_height {
        let ch = if row == thumb_pos { "#" } else { "|" };
        let style = if row == thumb_pos {
            Style::default().fg(Color::White)
        } else {
            track_style
        };
        frame.render_widget(
            Paragraph::new(Span::styled(ch, style)),
            Rect {
                x: area.x,
                y: area.y + 1 + row as u16,
                width: 1,
                height: 1,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two machines + plenty of room: configured graph rows wins (no shrink).
    #[test]
    fn summary_layout_two_machines_uses_configured() {
        assert_eq!(
            summary_layout(50, 2, 9),
            SummaryLayout::Expanded { graph_rows: 9 }
        );
    }

    /// Four machines on a typical 40-row metrics area: graph_rows shrinks
    /// from the configured 9 to whatever fits all four. This is the case
    /// Shawn hit: with 9 rows × 4 = 44 the expanded view bailed and fell
    /// back to compact; now it adapts.
    #[test]
    fn summary_layout_four_machines_shrinks_to_fit() {
        let layout = summary_layout(40, 4, 9);
        match layout {
            SummaryLayout::Expanded { graph_rows } => {
                assert!(graph_rows >= 3, "fit must clear the MIN_GRAPH_ROWS floor: {graph_rows}");
                // 4 machines × (graph_rows + 2) must still fit under the
                // available height (1 reserved for the header).
                assert!(
                    4 * (graph_rows + 2) <= 39,
                    "{graph_rows}-row blocks × 4 must fit in 39 rows"
                );
            }
            SummaryLayout::Compact => panic!("expected expanded for 40-row area with 4 machines"),
        }
    }

    /// Tight area where even MIN_GRAPH_ROWS won't fit all machines → compact.
    /// This guards the fallback path so 8-machine on a small SSH terminal
    /// degrades gracefully instead of cramming illegible 1-row graphs.
    #[test]
    fn summary_layout_falls_back_to_compact_when_too_tight() {
        // 4 machines, area only 12 rows tall: (12-1)/4 = 2 per machine,
        // minus 2 borders = 0 graph rows. Compact wins.
        assert_eq!(summary_layout(12, 4, 9), SummaryLayout::Compact);
    }

    /// Empty machine list (boot, no Prometheus data yet) must not divide
    /// by zero or produce a garbage Expanded layout — fall back to compact.
    #[test]
    fn summary_layout_zero_machines_is_compact() {
        assert_eq!(summary_layout(40, 0, 9), SummaryLayout::Compact);
    }
}
