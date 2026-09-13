// Copyright 2026 The Gilamonster Authors
// SPDX-License-Identifier: Apache-2.0

//! Full btop-style layout for a single machine sub-tab.
//!
//! Layout (with GPU):
//! ```text
//! ┌──── CPU ────────────────────┐┌── MEM ──────┐
//! │ per-core HeatGraph (local)  ││ mem graph    │
//! │ overall CPU HeatGraph       ││ disk meters  │
//! ├──── NET ────────────────────┤├── GPU ───────┤
//! │ RX HeatGraph (green)        ││ util graph   │
//! │ TX HeatGraph (cyan)         ││ vram meter   │
//! │                             ││ temp graph   │
//! ├──── PROC ───────────────────┴┴──────────────┤
//! │ PID  USER  CPU%  MEM%  COMMAND              │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! Without GPU the right column is MEM only (taller) and NET spans full width.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Direction;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, MachineState, ProcFilter, ProcSort};
use crate::event::MachineMetrics;
use crate::ui::metrics::{draw_graph, draw_graph_inverted, draw_heat_meter, format_bytes, value_color};

/// Draw a full btop-style view for the currently selected machine.
pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let idx = app.active_machine_idx;
    let metrics = match app.metrics.machines.get(idx) {
        Some(m) => m,
        None => {
            let msg = Paragraph::new("no machine data")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, area);
            return;
        }
    };
    let ms = match app.machine_states.get(idx) {
        Some(ms) => ms,
        None => return,
    };

    if ms.has_gpu {
        draw_with_gpu(frame, app, metrics, ms, area);
    } else {
        draw_without_gpu(frame, app, metrics, ms, area);
    }
}

/// Layout with GPU: 3 rows (top: CPU+MEM, mid: NET+GPU, bottom: PROC).
fn draw_with_gpu(frame: &mut Frame, app: &App, m: &MachineMetrics, ms: &MachineState, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35), // CPU + MEM
            Constraint::Percentage(35), // NET + GPU
            Constraint::Min(5),         // PROC
        ])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[0]);

    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(rows[1]);

    draw_cpu_box(frame, m, ms, top[0]);
    draw_mem_box(frame, m, ms, top[1]);
    let net_max_kbps = app.effective_net_max_kbps();
    draw_net_box(frame, m, net_max_kbps, mid[0]);
    draw_gpu_box(frame, m, mid[1]);
    draw_proc_box(frame, ms, &app.current_username, rows[2]);
}

/// Layout without GPU: 2 rows (top: CPU+MEM+NET 3-col, bottom: PROC/POD).
fn draw_without_gpu(frame: &mut Frame, app: &App, m: &MachineMetrics, ms: &MachineState, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // CPU + MEM + NET (3 columns)
            Constraint::Min(5),         // PROC/POD
        ])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34), // CPU
            Constraint::Percentage(33), // MEM
            Constraint::Percentage(33), // NET
        ])
        .split(rows[0]);

    draw_cpu_box(frame, m, ms, top[0]);
    draw_mem_box(frame, m, ms, top[1]);
    let net_max_kbps = app.effective_net_max_kbps();
    draw_net_box(frame, m, net_max_kbps, top[2]);
    draw_proc_box(frame, ms, &app.current_username, rows[1]);
}

/// CPU box — per-core HeatGraph rows (local) or overall HeatGraph (remote).
fn draw_cpu_box(frame: &mut Frame, m: &MachineMetrics, ms: &MachineState, area: Rect) {
    let temp_str = m
        .cpu_temp_c
        .map(|t| format!(" | Temp: {:.0}°C", t))
        .unwrap_or_default();
    let model_str = if m.cpu_model_name.is_empty() {
        String::new()
    } else {
        format!(" | {}", m.cpu_model_name)
    };
    let title = format!(" CPU: {:.0}%{}{} ", m.cpu_percent, temp_str, model_str);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(value_color(m.cpu_percent / 100.0)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if ms.is_local && !ms.cpu_per_core.is_empty() {
        // Per-core HeatGraph rows
        draw_cpu_cores_inner(frame, &ms.cpu_per_core, &ms.cpu_core_histories, inner);
    } else {
        // Remote: overall CPU HeatGraph
        draw_graph(frame, &m.cpu_history, 100, Color::Cyan, inner);
    }
}

/// Render per-core CPU as one HeatGraph row per core.
fn draw_cpu_cores_inner(
    frame: &mut Frame,
    cores: &[f64],
    histories: &[Vec<f64>],
    area: Rect,
) {
    if cores.is_empty() {
        return;
    }

    let constraints: Vec<Constraint> = cores.iter().map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let blocks: &[char] = &[' ', '░', '▒', '█'];

    for (i, &usage) in cores.iter().enumerate() {
        if i >= rows.len() {
            break;
        }
        let current_color = value_color(usage / 100.0);
        let history = histories.get(i).cloned().unwrap_or_default();
        let data: Vec<u64> = history.iter().map(|v| *v as u64).collect();

        let bar_width = area.width.saturating_sub(8) as usize;
        let mut spans: Vec<Span> = Vec::with_capacity(bar_width + 4);
        spans.push(Span::styled(
            format!("{i:>2} "),
            Style::default().fg(Color::DarkGray),
        ));

        if !data.is_empty() {
            let points: Vec<&u64> = data.iter().rev().take(bar_width).collect::<Vec<_>>();
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

/// MEM box — memory HeatGraph + disk HeatMeters.
fn draw_mem_box(frame: &mut Frame, m: &MachineMetrics, ms: &MachineState, area: Rect) {
    let mem_label = format!(
        " MEM {:.1}/{:.0}G {:.0}% ",
        m.memory_used_gb, m.memory_total_gb, m.memory_percent
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(mem_label)
        .border_style(Style::default().fg(value_color(m.memory_percent / 100.0)));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split: memory graph on top, separator, disk label, disk meters below
    let disk_count = ms.disks.len().max(1);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),                           // MEM graph
            Constraint::Length(1),                         // separator + disk label
            Constraint::Length(disk_count as u16),         // disk bars
        ])
        .split(inner);

    draw_graph(frame, &m.mem_history, 100, Color::Blue, rows[0]);

    // Separator line between MEM and DISK
    let sep = Line::from(Span::styled(
        format!(" {} DISK {}", "─".repeat(3), "─".repeat(rows[1].width.saturating_sub(10) as usize)),
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(Paragraph::new(sep), rows[1]);

    // Disk heat meters
    if !ms.disks.is_empty() {
        let disk_constraints: Vec<Constraint> = ms.disks.iter().map(|_| Constraint::Length(1)).collect();
        let disk_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(disk_constraints)
            .split(rows[2]);

        for (i, disk) in ms.disks.iter().enumerate() {
            if i >= disk_rows.len() {
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
            draw_heat_meter(frame, &label_short, pct, &right, disk_rows[i]);
        }
    } else {
        // Remote machine with only aggregate disk data
        if m.disk_total_gb > 0.0 {
            let disk_used = m.disk_total_gb - m.disk_free_gb;
            let disk_pct = (disk_used / m.disk_total_gb) * 100.0;
            let right = format!("{:.0}/{:.0}G", disk_used, m.disk_total_gb);
            draw_heat_meter(frame, "/", disk_pct, &right, rows[2]);
        }
    }
}

/// NET box — HeatButterflyGraph: TX grows down from center, RX grows up.
/// The two graphs meet at the midline creating "butterfly wings".
fn draw_net_box(frame: &mut Frame, m: &MachineMetrics, net_max_kbps: u64, area: Rect) {
    let nic_str = if m.nic_names.is_empty() {
        String::new()
    } else {
        format!("{} | ", m.nic_names.join(" "))
    };
    let title = format!(
        " NET {}TX:{} RX:{} ",
        nic_str,
        format_bytes(m.net_tx_bytes_sec),
        format_bytes(m.net_rx_bytes_sec),
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // TX (grows down from top)
            Constraint::Percentage(50), // RX (grows up from bottom, inverted)
        ])
        .split(inner);

    // TX: normal graph (grows down from top = default orientation)
    draw_graph(frame, &m.net_tx_history, net_max_kbps, Color::Cyan, rows[0]);
    // RX: inverted graph (grows up from bottom)
    draw_graph_inverted(frame, &m.net_rx_history, net_max_kbps, Color::Green, rows[1]);
}

/// GPU box — utilization HeatGraph, VRAM HeatMeter, temperature HeatGraph.
fn draw_gpu_box(frame: &mut Frame, m: &MachineMetrics, area: Rect) {
    let gpu_pct = m.gpu_percent.unwrap_or(0.0);
    let temp_str = m
        .gpu_temp_c
        .map(|t| format!(" | Temp: {:.0}°C", t))
        .unwrap_or_default();
    let mem_str = match (m.gpu_memory_used_mb, m.gpu_memory_total_mb) {
        (Some(used), Some(total)) => format!(" | Mem: {:.1}/{:.1}GB", used / 1024.0, total / 1024.0),
        _ => String::new(),
    };
    let model_str = if m.gpu_model_name.is_empty() {
        String::new()
    } else {
        format!(" | {}", m.gpu_model_name)
    };
    let title = format!(" GPU: {:.0}%{}{}{} ", gpu_pct, temp_str, mem_str, model_str);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Magenta));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // HeatButterfly: Util on top (grows down), VRAM inverted on bottom (grows up)
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // UTL graph (grows down)
            Constraint::Percentage(50), // VRAM graph (grows up, inverted)
        ])
        .split(inner);

    draw_graph(frame, &m.gpu_history, 100, Color::Magenta, rows[0]);
    draw_graph_inverted(frame, &m.gpu_mem_history, 100, Color::LightMagenta, rows[1]);
}

/// PROC box — scrollable process table (local machine only).
fn draw_proc_box(frame: &mut Frame, ms: &MachineState, current_user: &str, area: Rect) {
    let sort_label = match ms.proc_sort {
        ProcSort::Cpu => "CPU%",
        ProcSort::Mem => "MEM%",
    };
    let filter_label = ms.proc_filter.label();
    let title = format!(" PROC [sort: {} | filter: {}] ", sort_label, filter_label);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if !ms.is_local {
        draw_pod_box(frame, ms, inner);
        return;
    }

    if ms.processes.is_empty() {
        let msg = Paragraph::new("  waiting for process data...")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    // Apply process filter
    let filtered: Vec<_> = ms
        .processes
        .iter()
        .filter(|p| match ms.proc_filter {
            ProcFilter::All => true,
            ProcFilter::CurrentUser => p.user == current_user,
            ProcFilter::HideSelf => p.user != current_user,
        })
        .collect();

    // Header
    let header_height = 1;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(1),
        ])
        .split(inner);

    let header = Line::from(vec![
        Span::styled(
            format!("{:<6}{:<9}{:>5} {:>5} {}", "PID", "USER", "CPU%", "MEM%", "COMMAND"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(header), rows[0]);

    // Process rows (scrollable)
    let visible_rows = rows[1].height as usize;
    let scroll = ms.proc_scroll.min(filtered.len().saturating_sub(visible_rows));

    let proc_constraints: Vec<Constraint> = (0..visible_rows)
        .map(|_| Constraint::Length(1))
        .collect();
    let proc_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(proc_constraints)
        .split(rows[1]);

    // How much width is left for the command column
    let fixed_cols = 6 + 9 + 6 + 6; // PID + USER + CPU% + MEM% + spaces
    let cmd_width = (inner.width as usize).saturating_sub(fixed_cols);

    for (i, row_area) in proc_rows.iter().enumerate() {
        let proc_idx = scroll + i;
        if proc_idx >= filtered.len() {
            break;
        }
        let p = filtered[proc_idx];
        let cpu_color = value_color(p.cpu_pct / 100.0);
        let mem_color = value_color(p.mem_pct / 100.0);

        let user_short = if p.user.len() > 8 {
            format!("{}.", &p.user[..7])
        } else {
            format!("{:<8}", p.user)
        };

        // Truncate command to fit, with ellipsis
        let cmd = if p.command.len() > cmd_width && cmd_width > 3 {
            format!("{}...", &p.command[..cmd_width - 3])
        } else {
            p.command.clone()
        };

        let line = Line::from(vec![
            Span::styled(format!("{:<6}", p.pid), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{user_short} "), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:>5.1}", p.cpu_pct), Style::default().fg(cpu_color)),
            Span::raw(" "),
            Span::styled(format!("{:>5.1}", p.mem_pct), Style::default().fg(mem_color)),
            Span::raw(" "),
            Span::styled(cmd, Style::default().fg(Color::White)),
        ]);
        frame.render_widget(Paragraph::new(line), *row_area);
    }
}

/// POD box — k8s pod list for remote machines (from cadvisor via Prometheus).
fn draw_pod_box(frame: &mut Frame, ms: &MachineState, area: Rect) {
    if ms.pods.is_empty() {
        let msg = Paragraph::new("  waiting for pod data (cadvisor)...")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, area);
        return;
    }

    // Header
    let header_height = 1;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(header_height), Constraint::Min(1)])
        .split(area);

    let header = Line::from(vec![Span::styled(
        format!(
            "{:<12}{:<32}{:>6} {:>8}",
            "NAMESPACE", "POD", "CPU", "MEM"
        ),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )]);
    frame.render_widget(Paragraph::new(header), rows[0]);

    let visible_rows = rows[1].height as usize;
    let scroll = ms.proc_scroll.min(ms.pods.len().saturating_sub(visible_rows));

    let pod_constraints: Vec<Constraint> = (0..visible_rows)
        .map(|_| Constraint::Length(1))
        .collect();
    let pod_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(pod_constraints)
        .split(rows[1]);

    for (i, row_area) in pod_rows.iter().enumerate() {
        let idx = scroll + i;
        if idx >= ms.pods.len() {
            break;
        }
        let pod = &ms.pods[idx];

        let ns = if pod.namespace.len() > 11 {
            format!("{}.", &pod.namespace[..10])
        } else {
            format!("{:<11}", pod.namespace)
        };

        let name = if pod.name.len() > 31 {
            format!("{}.", &pod.name[..30])
        } else {
            format!("{:<31}", pod.name)
        };

        // CPU in millicores (e.g., 234m) or cores (e.g., 1.2)
        let cpu_str = if pod.cpu_cores < 1.0 {
            format!("{:.0}m", pod.cpu_cores * 1000.0)
        } else {
            format!("{:.1}", pod.cpu_cores)
        };

        // Memory in Mi or Gi
        let mem_mb = pod.memory_bytes as f64 / 1_048_576.0;
        let mem_str = if mem_mb >= 1024.0 {
            format!("{:.1}Gi", mem_mb / 1024.0)
        } else {
            format!("{:.0}Mi", mem_mb)
        };

        let cpu_color = value_color((pod.cpu_cores * 10.0).min(1.0)); // 100m = green, 1000m = red
        let mem_color = value_color((mem_mb / 2048.0).min(1.0)); // 2Gi = red

        let line = Line::from(vec![
            Span::styled(format!("{ns} "), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{name} "), Style::default().fg(Color::White)),
            Span::styled(format!("{:>5}", cpu_str), Style::default().fg(cpu_color)),
            Span::raw(" "),
            Span::styled(format!("{:>7}", mem_str), Style::default().fg(mem_color)),
        ]);
        frame.render_widget(Paragraph::new(line), *row_area);
    }
}
