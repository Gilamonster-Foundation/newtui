// Copyright 2026 The Gilamonster Authors
// SPDX-License-Identifier: Apache-2.0

//! Settings tab — live-editable configuration display with sub-tabs.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::prelude::Direction;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::app::{App, SettingsSubTab};

/// What kind of value a setting holds (for parsing and toggling).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    Bool,
    U32,
    U64,
    Usize,
    F64,
    /// A trigger checkbox — toggled on/off.
    Trigger,
    /// Cycle through predefined options on Enter.
    Cycle(&'static [&'static str]),
}

/// One row in the settings list.
struct SettingItem {
    label: &'static str,
    value: String,
    editable: bool,
    field_type: FieldType,
    field_id: &'static str,
}

// espeak voice options
const ESPEAK_VOICES: &[&str] = &["en-us", "en-gb", "en-gb-scotland", "en-gb-x-rp", "en-029", "en-us-nyc", "en"];
// piper model options (common English models)
const PIPER_MODELS: &[&str] = &["en_US-amy-medium", "en_US-lessac-medium", "en_US-ryan-medium", "en_GB-alan-medium"];

/// Build the setting items for the Voice sub-tab.
fn build_voice_items(app: &App) -> Vec<SettingItem> {
    let cfg = &app.settings_config;
    let mut items = vec![
        // --- Hardware section ---
        SettingItem {
            label: "Enabled",
            value: if cfg.voice.enabled { "yes" } else { "no" }.into(),
            editable: true,
            field_type: FieldType::Bool,
            field_id: "voice.enabled",
        },
        SettingItem {
            label: "ALSA playback",
            value: cfg.voice.alsa_device.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "voice.alsa_device",
        },
        SettingItem {
            label: "Capture device",
            value: cfg.voice.capture_device.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "voice.capture_device",
        },
        SettingItem {
            label: "Whisper model",
            value: cfg.voice.whisper_model.clone(),
            editable: true,
            field_type: FieldType::Cycle(&["tiny", "base", "small"]),
            field_id: "voice.whisper_model",
        },
        SettingItem {
            label: "Mic max secs",
            value: cfg.voice.record_duration.to_string(),
            editable: true,
            field_type: FieldType::U32,
            field_id: "voice.record_duration",
        },
        SettingItem {
            label: "Cooldown secs",
            value: cfg.voice.voice_cooldown_secs.to_string(),
            editable: true,
            field_type: FieldType::U64,
            field_id: "voice.voice_cooldown_secs",
        },
        SettingItem {
            label: "Python path",
            value: cfg.voice.python_path.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "voice.python_path",
        },
        // --- Speaking behavior + sound check ---
        SettingItem {
            label: "Speak replies",
            value: if cfg.voice.always_talk { "on" } else { "off" }.into(),
            editable: true,
            field_type: FieldType::Bool,
            field_id: "voice.always_talk",
        },
        SettingItem {
            label: "Volume %",
            value: app
                .volume_percent
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".into()),
            editable: true,
            field_type: FieldType::U32,
            field_id: "volume",
        },
        SettingItem {
            label: "Test speaker",
            value: "▶ play".into(),
            editable: true,
            field_type: FieldType::Trigger,
            field_id: "voice.test_speaker",
        },
        SettingItem {
            label: "Test mic",
            value: "▶ record".into(),
            editable: true,
            field_type: FieldType::Trigger,
            field_id: "voice.test_mic",
        },
    ];

    // --- Monty section ---
    items.push(SettingItem {
        label: "Monty engine",
        value: cfg.voice.monty.engine.clone(),
        editable: true,
        field_type: FieldType::Cycle(&["espeak", "piper", "gila"]),
        field_id: "monty.engine",
    });
    match cfg.voice.monty.engine.as_str() {
        "piper" => {
            items.push(SettingItem {
                label: "Monty model",
                value: cfg.voice.monty.piper_model.clone(),
                editable: true,
                field_type: FieldType::Cycle(PIPER_MODELS),
                field_id: "monty.piper_model",
            });
        }
        "espeak" | _ if cfg.voice.monty.engine != "gila" => {
            items.push(SettingItem {
                label: "Monty voice",
                value: cfg.voice.monty.espeak_voice.clone(),
                editable: true,
                field_type: FieldType::Cycle(ESPEAK_VOICES),
                field_id: "monty.espeak_voice",
            });
            items.push(SettingItem {
                label: "Monty speed",
                value: cfg.voice.monty.espeak_speed.to_string(),
                editable: true,
                field_type: FieldType::U32,
                field_id: "monty.espeak_speed",
            });
            items.push(SettingItem {
                label: "Monty pitch",
                value: cfg.voice.monty.espeak_pitch.to_string(),
                editable: true,
                field_type: FieldType::U32,
                field_id: "monty.espeak_pitch",
            });
        }
        _ => {} // gila engine has no extra params
    }

    // --- External section ---
    items.push(SettingItem {
        label: "Ext engine",
        value: cfg.voice.external.engine.clone(),
        editable: true,
        field_type: FieldType::Cycle(&["espeak", "piper", "gila"]),
        field_id: "external.engine",
    });
    match cfg.voice.external.engine.as_str() {
        "piper" => {
            items.push(SettingItem {
                label: "Ext model",
                value: cfg.voice.external.piper_model.clone(),
                editable: true,
                field_type: FieldType::Cycle(PIPER_MODELS),
                field_id: "external.piper_model",
            });
        }
        "espeak" | _ if cfg.voice.external.engine != "gila" => {
            items.push(SettingItem {
                label: "Ext voice",
                value: cfg.voice.external.espeak_voice.clone(),
                editable: true,
                field_type: FieldType::Cycle(ESPEAK_VOICES),
                field_id: "external.espeak_voice",
            });
            items.push(SettingItem {
                label: "Ext speed",
                value: cfg.voice.external.espeak_speed.to_string(),
                editable: true,
                field_type: FieldType::U32,
                field_id: "external.espeak_speed",
            });
            items.push(SettingItem {
                label: "Ext pitch",
                value: cfg.voice.external.espeak_pitch.to_string(),
                editable: true,
                field_type: FieldType::U32,
                field_id: "external.espeak_pitch",
            });
        }
        _ => {} // gila engine has no extra params
    }

    items
}

/// Build the setting items for the Display sub-tab.
fn build_display_items(app: &App) -> Vec<SettingItem> {
    let cfg = &app.settings_config;
    let mut items = vec![
        SettingItem {
            label: "Persona",
            value: cfg.persona.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "persona",
        },
        SettingItem {
            label: "Short name",
            value: cfg.persona_short.clone().unwrap_or_default(),
            editable: true,
            field_type: FieldType::String,
            field_id: "persona_short",
        },
        SettingItem {
            label: "Nickname (user)",
            value: cfg.user_nickname.clone().unwrap_or_default(),
            editable: true,
            field_type: FieldType::String,
            field_id: "user_nickname",
        },
        SettingItem {
            label: "Refresh interval",
            value: cfg.refresh_interval_secs.to_string(),
            editable: true,
            field_type: FieldType::U64,
            field_id: "refresh_interval_secs",
        },
        SettingItem {
            label: "Summary rows",
            value: cfg.summary_graph_rows.to_string(),
            editable: true,
            field_type: FieldType::Usize,
            field_id: "summary_graph_rows",
        },
        SettingItem {
            label: "CI history hrs",
            value: cfg.ci_history_hours.to_string(),
            editable: true,
            field_type: FieldType::Usize,
            field_id: "ci_history_hours",
        },
        SettingItem {
            label: "CI poll secs",
            value: cfg.ci_poll_interval_secs.to_string(),
            editable: true,
            field_type: FieldType::U64,
            field_id: "ci_poll_interval_secs",
        },
        SettingItem {
            label: "CI graph rows",
            value: cfg.ci_graph_rows.to_string(),
            editable: true,
            field_type: FieldType::Usize,
            field_id: "ci_graph_rows",
        },
        SettingItem {
            label: "Net label width",
            value: cfg.net_label_width.to_string(),
            editable: true,
            field_type: FieldType::Usize,
            field_id: "net_label_width",
        },
        SettingItem {
            label: "Net max bytes",
            value: format!("{:.0}", cfg.net_max_bytes),
            editable: true,
            field_type: FieldType::F64,
            field_id: "net_max_bytes",
        },
        SettingItem {
            label: "Proc filter",
            value: cfg.proc_filter.clone(),
            editable: true,
            field_type: FieldType::Cycle(&["all", "current_user", "hide_self"]),
            field_id: "proc_filter",
        },
    ];

    items.push(SettingItem {
        label: "Speak reactions",
        value: if cfg.attention.speak { "on" } else { "off" }.into(),
        editable: true,
        field_type: FieldType::Bool,
        field_id: "attention.speak",
    });
    items.push(SettingItem {
        label: "Disk warn %",
        value: format!("{:.0}", cfg.attention.disk_warn_pct),
        editable: true,
        field_type: FieldType::F64,
        field_id: "attention.disk_warn_pct",
    });

    // Attention triggers as individual toggles
    let triggers = &cfg.attention_triggers;
    for (name, field_id) in &[
        ("cpu", "trigger:cpu"),
        ("memory", "trigger:memory"),
        ("gpu", "trigger:gpu"),
        ("network", "trigger:network"),
    ] {
        let on = triggers.iter().any(|t| t == name);
        items.push(SettingItem {
            label: match *name {
                "cpu" => "Trigger: cpu",
                "memory" => "Trigger: memory",
                "gpu" => "Trigger: gpu",
                "network" => "Trigger: network",
                _ => "Trigger: ???",
            },
            value: if on { "on" } else { "off" }.into(),
            editable: true,
            field_type: FieldType::Trigger,
            field_id,
        });
    }

    items.push(SettingItem {
        label: "Max history",
        value: cfg.max_history.to_string(),
        editable: true,
        field_type: FieldType::Usize,
        field_id: "max_history",
    });
    items.push(SettingItem {
        label: "Max attn history",
        value: cfg.max_attention_history.to_string(),
        editable: true,
        field_type: FieldType::Usize,
        field_id: "max_attention_history",
    });

    items
}

/// Build the setting items for the Connections sub-tab.
fn build_connections_items(app: &App) -> Vec<SettingItem> {
    let cfg = &app.settings_config;
    vec![
        SettingItem {
            label: "NATS URL",
            value: cfg.nats.url.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "nats.url",
        },
        SettingItem {
            label: "NATS nkey seed",
            value: cfg.nats.nkey_seed.clone().unwrap_or_default(),
            editable: true,
            field_type: FieldType::String,
            field_id: "nats.nkey_seed",
        },
        SettingItem {
            label: "Prometheus URL",
            value: cfg.prometheus.url.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "prometheus.url",
        },
        SettingItem {
            label: "LLM endpoint",
            value: cfg.llm.endpoint.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "llm.endpoint",
        },
        SettingItem {
            label: "LLM model",
            value: cfg.llm.model.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "llm.model",
        },
        SettingItem {
            label: "LLM system prompt",
            value: cfg.llm.system_prompt.clone(),
            editable: true,
            field_type: FieldType::String,
            field_id: "llm.system_prompt",
        },
        SettingItem {
            label: "LLM timeout secs",
            value: cfg.llm.timeout_secs.to_string(),
            editable: true,
            field_type: FieldType::U64,
            field_id: "llm.timeout_secs",
        },
        SettingItem {
            label: "GitHub user",
            value: cfg.github_username.clone().unwrap_or_default(),
            editable: true,
            field_type: FieldType::String,
            field_id: "github_username",
        },
    ]
}

/// Build the setting items for the Machines sub-tab (read-only table).
fn build_machines_items(app: &App) -> Vec<SettingItem> {
    let cfg = &app.settings_config;
    let mut items = Vec::new();
    for mc in &cfg.machines {
        let ms = app
            .machine_states
            .iter()
            .find(|ms| ms.name == mc.name);
        let connected = ms.map(|ms| ms.connected).unwrap_or(false);
        let pod_count = ms.map(|ms| ms.pods.len()).unwrap_or(0);
        let proc_count = ms.map(|ms| ms.processes.len()).unwrap_or(0);
        items.push(SettingItem {
            label: "Machine",
            value: format!(
                "{:<12} {:<24} gpu:{:<5} {:>3} pods {:>4} procs  {}",
                mc.name,
                mc.instance,
                if mc.has_gpu { "yes" } else { "no" },
                pod_count,
                proc_count,
                if connected { "connected" } else { "---" },
            ),
            editable: false,
            field_type: FieldType::String,
            field_id: "machine:readonly",
        });
    }
    items
}

/// Build items for the active sub-tab.
fn build_items(app: &App) -> Vec<SettingItem> {
    match app.active_settings_sub_tab {
        SettingsSubTab::Voice => build_voice_items(app),
        SettingsSubTab::Display => build_display_items(app),
        SettingsSubTab::Connections => build_connections_items(app),
        SettingsSubTab::Machines => build_machines_items(app),
    }
}

/// Count display lines for the active sub-tab.
pub fn total_display_lines(app: &App) -> usize {
    build_items(app).len()
}

/// Return the field type of the currently selected setting.
pub fn selected_field_type(app: &App) -> FieldType {
    let items = build_items(app);
    items
        .get(app.settings_selected)
        .map(|item| item.field_type)
        .unwrap_or(FieldType::String)
}

/// Return whether the currently selected setting is editable.
pub fn selected_is_editable(app: &App) -> bool {
    let items = build_items(app);
    items
        .get(app.settings_selected)
        .map(|item| item.editable)
        .unwrap_or(false)
}

/// Return the current value of the selected setting (for populating the edit buffer).
pub fn selected_value(app: &App) -> String {
    let items = build_items(app);
    items
        .get(app.settings_selected)
        .map(|item| item.value.clone())
        .unwrap_or_default()
}

/// Return the total number of setting items (for bounds checking).
pub fn item_count(app: &App) -> usize {
    build_items(app).len()
}

/// Apply an edited value back to settings_config (and live App fields).
///
/// This is called from app.rs when the user presses Enter to confirm an edit.
/// Apply a value change by field_id directly (used by MCP SetConfig).
/// Updates both settings_config and live App state.
pub fn apply_value_by_id(app: &mut App, field_id: &str, value: &str) {
    apply_value_inner(app, field_id, value);
}

/// Dispatches on the field_id of the currently selected item.
pub fn apply_value(app: &mut App, value: &str) {
    let idx = app.settings_selected;

    // Build the items for the active sub-tab to find the field_id.
    let field_id = {
        let items = build_items(app);
        match items.get(idx) {
            Some(item) => item.field_id,
            None => return,
        }
    };

    apply_value_inner(app, field_id, value);
}

fn apply_value_inner(app: &mut App, field_id: &str, value: &str) {
    match field_id {
        "persona" => {
            app.settings_config.persona = value.to_string();
            app.persona_name = value.to_string();
        }
        "persona_short" => {
            let v = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
            app.settings_config.persona_short = v.clone();
            app.persona_short = v;
        }
        "user_nickname" => {
            let v = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
            app.settings_config.user_nickname = v.clone();
            app.user_nickname = v;
        }
        // --- Hardware fields ---
        "voice.enabled" => {
            let b = if value.is_empty() {
                !app.settings_config.voice.enabled
            } else {
                matches!(value, "yes" | "true" | "1")
            };
            app.settings_config.voice.enabled = b;
            app.voice_hardware.enabled = b;
        }
        "voice.alsa_device" => {
            app.settings_config.voice.alsa_device = value.to_string();
            app.voice_hardware.alsa_device = value.to_string();
        }
        "voice.capture_device" => {
            app.settings_config.voice.capture_device = value.to_string();
            app.voice_hardware.capture_device = value.to_string();
        }
        "voice.whisper_model" => {
            let options = &["tiny", "base", "small"];
            let new_val = if value.is_empty() {
                cycle_value(&app.settings_config.voice.whisper_model, options)
            } else {
                value.to_string()
            };
            app.settings_config.voice.whisper_model = new_val.clone();
            app.voice_hardware.whisper_model = new_val;
        }
        "voice.record_duration" => {
            if let Ok(n) = value.parse::<u32>() {
                app.settings_config.voice.record_duration = n;
                app.voice_hardware.record_duration = n;
            }
        }
        "voice.voice_cooldown_secs" => {
            if let Ok(n) = value.parse::<u64>() {
                app.settings_config.voice.voice_cooldown_secs = n;
                app.voice_hardware.voice_cooldown_secs = n;
            }
        }
        "voice.python_path" => {
            app.settings_config.voice.python_path = value.to_string();
            app.voice_hardware.python_path = value.to_string();
        }
        "voice.always_talk" => {
            let on = if value.is_empty() {
                !app.settings_config.voice.always_talk
            } else {
                matches!(value, "yes" | "true" | "on" | "1")
            };
            app.settings_config.voice.always_talk = on;
            app.always_talk = on;
        }
        "volume" => {
            if let Ok(n) = value.parse::<u32>() {
                app.execute_tui_command(crate::event::TuiCommand::SetVolume(n));
            }
        }
        "voice.test_speaker" => {
            crate::data::voice::speak(
                &app.voice_hardware,
                &app.monty_engine,
                "Sound check. One, two. Can you hear me, Boss?",
            );
            app.reaction_text = "Testing speaker…".to_string();
            app.reaction_time = std::time::Instant::now();
        }
        "voice.test_mic" => {
            // Record once and report what was heard (don't send to the LLM).
            app.mic_test_pending = true;
            app.toggle_voice_recording();
            app.reaction_text = "Mic test: speak, then tap Space to stop".to_string();
            app.reaction_time = std::time::Instant::now();
        }
        // --- Monty persona fields ---
        "monty.engine" => {
            let options = &["espeak", "piper", "gila"];
            let new_val = if value.is_empty() {
                cycle_value(&app.settings_config.voice.monty.engine, options)
            } else {
                value.to_string()
            };
            app.settings_config.voice.monty.engine = new_val.clone();
            app.monty_engine = rebuild_engine(&app.settings_config.voice.monty);
            crate::data::voice::speak(&app.voice_hardware, &app.monty_engine, &format!("Monty on {new_val}"));
        }
        "monty.espeak_voice" => {
            let new_val = if value.is_empty() {
                cycle_value(&app.settings_config.voice.monty.espeak_voice, ESPEAK_VOICES)
            } else { value.to_string() };
            app.settings_config.voice.monty.espeak_voice = new_val.clone();
            app.monty_engine = rebuild_engine(&app.settings_config.voice.monty);
            crate::data::voice::speak(&app.voice_hardware, &app.monty_engine, &format!("Hello, {new_val}"));
        }
        "monty.espeak_speed" => {
            if let Ok(n) = value.parse::<u32>() {
                app.settings_config.voice.monty.espeak_speed = n;
                app.monty_engine = rebuild_engine(&app.settings_config.voice.monty);
                crate::data::voice::speak(&app.voice_hardware, &app.monty_engine, "Testing speed");
            }
        }
        "monty.espeak_pitch" => {
            if let Ok(n) = value.parse::<u32>() {
                app.settings_config.voice.monty.espeak_pitch = n;
                app.monty_engine = rebuild_engine(&app.settings_config.voice.monty);
                crate::data::voice::speak(&app.voice_hardware, &app.monty_engine, "Testing pitch");
            }
        }
        "monty.piper_model" => {
            let new_val = if value.is_empty() {
                cycle_value(&app.settings_config.voice.monty.piper_model, PIPER_MODELS)
            } else { value.to_string() };
            let label = new_val.split('-').nth(1).unwrap_or(&new_val).to_string();
            app.settings_config.voice.monty.piper_model = new_val;
            app.monty_engine = rebuild_engine(&app.settings_config.voice.monty);
            crate::data::voice::speak(&app.voice_hardware, &app.monty_engine, &format!("Hello, I am {label}"));
        }
        // --- External persona fields ---
        "external.engine" => {
            let options = &["espeak", "piper", "gila"];
            let new_val = if value.is_empty() {
                cycle_value(&app.settings_config.voice.external.engine, options)
            } else {
                value.to_string()
            };
            app.settings_config.voice.external.engine = new_val.clone();
            app.external_engine = rebuild_engine(&app.settings_config.voice.external);
            crate::data::voice::speak(&app.voice_hardware, &app.external_engine, &format!("Claude on {new_val}"));
        }
        "external.espeak_voice" => {
            let new_val = if value.is_empty() {
                cycle_value(&app.settings_config.voice.external.espeak_voice, ESPEAK_VOICES)
            } else { value.to_string() };
            app.settings_config.voice.external.espeak_voice = new_val.clone();
            app.external_engine = rebuild_engine(&app.settings_config.voice.external);
            crate::data::voice::speak(&app.voice_hardware, &app.external_engine, &format!("Hello, {new_val}"));
        }
        "external.espeak_speed" => {
            if let Ok(n) = value.parse::<u32>() {
                app.settings_config.voice.external.espeak_speed = n;
                app.external_engine = rebuild_engine(&app.settings_config.voice.external);
                crate::data::voice::speak(&app.voice_hardware, &app.external_engine, "Testing speed");
            }
        }
        "external.espeak_pitch" => {
            if let Ok(n) = value.parse::<u32>() {
                app.settings_config.voice.external.espeak_pitch = n;
                app.external_engine = rebuild_engine(&app.settings_config.voice.external);
                crate::data::voice::speak(&app.voice_hardware, &app.external_engine, "Testing pitch");
            }
        }
        "external.piper_model" => {
            let new_val = if value.is_empty() {
                cycle_value(&app.settings_config.voice.external.piper_model, PIPER_MODELS)
            } else { value.to_string() };
            app.settings_config.voice.external.piper_model = new_val;
            app.external_engine = rebuild_engine(&app.settings_config.voice.external);
        }
        "refresh_interval_secs" => {
            if let Ok(n) = value.parse::<u64>() {
                app.settings_config.refresh_interval_secs = n;
            }
        }
        "summary_graph_rows" => {
            if let Ok(n) = value.parse::<usize>() {
                app.settings_config.summary_graph_rows = n;
                app.summary_graph_rows = n;
            }
        }
        "ci_history_hours" => {
            if let Ok(n) = value.parse::<usize>() {
                app.settings_config.ci_history_hours = n;
            }
        }
        "ci_poll_interval_secs" => {
            if let Ok(n) = value.parse::<u64>() {
                app.settings_config.ci_poll_interval_secs = n;
            }
        }
        "ci_graph_rows" => {
            if let Ok(n) = value.parse::<usize>() {
                app.settings_config.ci_graph_rows = n;
                app.ci_graph_rows = n;
            }
        }
        "net_label_width" => {
            if let Ok(n) = value.parse::<usize>() {
                app.settings_config.net_label_width = n;
                app.net_label_width = n;
            }
        }
        "net_max_bytes" => {
            if let Ok(n) = value.parse::<f64>() {
                app.settings_config.net_max_bytes = n;
                app.net_max_bytes = n;
            }
        }
        "proc_filter" => {
            let options = &["all", "current_user", "hide_self"];
            let new_val = if value.is_empty() {
                cycle_value(&app.settings_config.proc_filter, options)
            } else {
                value.to_string()
            };
            app.settings_config.proc_filter = new_val;
        }
        "attention.speak" => {
            // Empty value = toggle (flip), so the Enter-toggle can turn it
            // back ON, not just off.
            let on = if value.is_empty() {
                !app.settings_config.attention.speak
            } else {
                matches!(value, "on" | "true" | "1" | "yes")
            };
            app.settings_config.attention.speak = on;
            app.attention_config.speak = on;
        }
        "attention.disk_warn_pct" => {
            if let Ok(n) = value.parse::<f64>() {
                app.settings_config.attention.disk_warn_pct = n;
                app.attention_config.disk_warn_pct = n;
            }
        }
        "trigger:cpu" | "trigger:memory" | "trigger:gpu" | "trigger:network" => {
            let trigger_name = field_id.strip_prefix("trigger:").unwrap_or("");
            let has_it = app
                .settings_config
                .attention_triggers
                .iter()
                .any(|t| t == trigger_name);
            if has_it {
                app.settings_config
                    .attention_triggers
                    .retain(|t| t != trigger_name);
            } else {
                app.settings_config
                    .attention_triggers
                    .push(trigger_name.to_string());
            }
            app.attention_triggers = crate::app::AttentionTriggers::from_config(
                &app.settings_config.attention_triggers,
            );
        }
        "max_history" => {
            if let Ok(n) = value.parse::<usize>() {
                app.settings_config.max_history = n;
                app.max_history = n;
            }
        }
        "max_attention_history" => {
            if let Ok(n) = value.parse::<usize>() {
                app.settings_config.max_attention_history = n;
                app.max_attention_history = n;
            }
        }
        "nats.url" => {
            app.settings_config.nats.url = value.to_string();
        }
        "nats.nkey_seed" => {
            let v = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
            app.settings_config.nats.nkey_seed = v;
        }
        "prometheus.url" => {
            app.settings_config.prometheus.url = value.to_string();
        }
        "llm.endpoint" => {
            app.settings_config.llm.endpoint = value.to_string();
            app.llm_config.endpoint = value.to_string();
        }
        "llm.model" => {
            app.settings_config.llm.model = value.to_string();
            app.llm_config.model = value.to_string();
        }
        "llm.system_prompt" => {
            app.settings_config.llm.system_prompt = value.to_string();
            app.llm_config.system_prompt = value.to_string();
        }
        "llm.timeout_secs" => {
            if let Ok(n) = value.parse::<u64>() {
                app.settings_config.llm.timeout_secs = n;
                app.llm_config.timeout_secs = n;
            }
        }
        "github_username" => {
            let v = if value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
            app.settings_config.github_username = v;
        }
        _ => {} // read-only or unknown
    }
}

/// Save the current settings_config to disk as TOML.
pub fn save_config(app: &App) -> Result<(), String> {
    let content = toml::to_string_pretty(&app.settings_config)
        .map_err(|e| format!("serialize error: {e}"))?;
    let path = std::path::Path::new("/etc/monitor-lizard/config.toml");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir error: {e}"))?;
    }
    std::fs::write(path, content).map_err(|e| format!("write error: {e}"))?;
    Ok(())
}

/// Draw the sub-tab bar for the Settings tab.
fn draw_sub_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = SettingsSubTab::all()
        .iter()
        .map(|t| Line::from(t.label()))
        .collect();
    let selected = SettingsSubTab::all()
        .iter()
        .position(|t| *t == app.active_settings_sub_tab)
        .unwrap_or(0);
    let tabs = Tabs::new(titles)
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
    frame.render_widget(tabs, area);
}

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let sub_tab_label = app.active_settings_sub_tab.label();
    let title = format!(
        " Settings [{}] (S:save Enter:edit Esc:cancel h/l:tab) ",
        sub_tab_label,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width < 10 {
        return;
    }

    // Split inner area: sub-tab bar (1 line) + content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    draw_sub_tab_bar(frame, app, chunks[0]);

    let content_area = chunks[1];
    let items = build_items(app);
    let label_width: u16 = 20;
    let visible_height = content_area.height as usize;

    // Find the display line for the selected item and handle scrolling
    let selected_line = app.settings_selected;
    let scroll = app.settings_scroll;
    let scroll = if selected_line < scroll {
        selected_line
    } else if selected_line >= scroll + visible_height {
        selected_line + 1 - visible_height
    } else {
        scroll
    };

    // Render visible items
    let mut lines_out: Vec<Line> = Vec::new();
    for (i, item) in items.iter().enumerate().skip(scroll).take(visible_height) {
        let is_selected = i == app.settings_selected;
        let is_editing = is_selected && app.settings_editing;

        let indicator = if is_selected { "> " } else { "  " };

        let label_text = format!(
            "{indicator}{:<width$}",
            format!("{}:", item.label),
            width = label_width as usize
        );

        let value_text = if is_editing {
            format!("[{}|]", app.settings_edit_value)
        } else if matches!(item.field_type, FieldType::Cycle(_)) {
            format!("< {} >", item.value)
        } else if matches!(item.field_type, FieldType::Bool) {
            if item.value == "true" || item.value == "yes" { "[x]".into() } else { "[ ]".into() }
        } else {
            format!("[{}]", item.value)
        };

        let label_style = if is_selected {
            Style::default().fg(Color::Green)
        } else if !item.editable {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Gray)
        };

        let value_style = if is_editing {
            Style::default().fg(Color::Yellow)
        } else if is_selected {
            Style::default().fg(Color::Green)
        } else if !item.editable {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        lines_out.push(Line::from(vec![
            Span::styled(label_text, label_style),
            Span::styled(value_text, value_style),
        ]));
    }

    let paragraph = Paragraph::new(lines_out);
    frame.render_widget(paragraph, content_area);
}

/// Build a type-safe TtsEngine from a PersonaVoice config (for live settings updates).
fn rebuild_engine(pv: &crate::config::PersonaVoice) -> crate::data::voice::TtsEngine {
    match pv.engine.as_str() {
        "piper" => crate::data::voice::TtsEngine::Piper(crate::data::voice::PiperVoice {
            model: pv.piper_model.clone(),
            path: pv.piper_path.clone(),
        }),
        "gila" => crate::data::voice::TtsEngine::Gila,
        _ => crate::data::voice::TtsEngine::Espeak(crate::data::voice::EspeakVoice {
            voice: pv.espeak_voice.clone(),
            speed: pv.espeak_speed,
            pitch: pv.espeak_pitch,
        }),
    }
}

/// Cycle to the next value in a list of options.
fn cycle_value(current: &str, options: &[&str]) -> String {
    let idx = options.iter().position(|&o| o == current).unwrap_or(0);
    let next = (idx + 1) % options.len();
    options[next].to_string()
}
