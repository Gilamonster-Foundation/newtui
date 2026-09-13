// Copyright 2026 The Gilamonster Authors
// SPDX-License-Identifier: Apache-2.0

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Direction;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Gauge};

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Budget ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(budget) = &app.budget else {
        let msg = ratatui::widgets::Paragraph::new("No budget data (daemon not running)")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);

    draw_gauge(
        frame,
        "Daily",
        budget.spent_today_usd,
        budget.daily_limit_usd,
        chunks[0],
    );
    draw_gauge(
        frame,
        "Weekly",
        budget.spent_this_week_usd,
        budget.weekly_limit_usd,
        chunks[1],
    );
    draw_gauge(
        frame,
        "Monthly",
        budget.spent_this_month_usd,
        budget.monthly_limit_usd,
        chunks[2],
    );
}

fn draw_gauge(frame: &mut Frame, label: &str, spent: f64, limit: f64, area: Rect) {
    let ratio = if limit > 0.0 {
        (spent / limit).min(1.0)
    } else {
        0.0
    };

    let color = if ratio >= 0.9 {
        Color::Red
    } else if ratio >= 0.7 {
        Color::Yellow
    } else {
        Color::Green
    };

    let gauge = Gauge::default()
        .block(Block::default().title(format!(" {label}: ${spent:.2} / ${limit:.2} ")))
        .gauge_style(Style::default().fg(color))
        .ratio(ratio);

    frame.render_widget(gauge, area);
}
