use crossbeam::channel::Sender;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use crate::params::{GlykonsHairParams, UserSampleRef};
use crate::pattern_preview::{build_pattern_preview, PatternEvent, PatternPreview};
use crate::samples::{load_wav_sample, sample_symbol_from_id};
use crate::timbres::TIMBRE_NAMES;
use crate::validation::{self, AudioMessage, SyntaxMode};

struct EditorState {
    selected_note: u8,
    text_buffer: String,
    tx: Sender<AudioMessage>,
    preview_playing: bool,
    search_query: String,
    linked_file: Option<std::path::PathBuf>,
    last_modified: std::time::SystemTime,
    syntax_mode: SyntaxMode,
    last_compile_error: Option<String>,
    sample_id_buffer: String,
    sample_path_buffer: String,
    sample_load_error: Option<String>,
    loaded_sample_symbols: HashSet<String>,
    pattern_preview_running: bool,
    pattern_preview_bpm: f32,
    pattern_preview_steps: usize,
}

struct DocItem {
    symbol: &'static str,
    desc: &'static str,
    example: &'static str,
}

struct Suggestion {
    label: &'static str,
    snippet: String,
}

const GLICOL_DOCS: &[DocItem] = &[
    DocItem {
        symbol: "out",
        desc: "Master output node.",
        example: "out: sin 220 >> mul 0.5 // Master output node",
    },
    DocItem {
        symbol: "sin",
        desc: "Sine wave oscillator.",
        example: "// sine wave at 440 Hz\nsin 440",
    },
    DocItem {
        symbol: "saw",
        desc: "Sawtooth wave oscillator.",
        example: "// sawtooth wave at 220 Hz\nsaw 220 >> mul 0.5",
    },
    DocItem {
        symbol: "sq",
        desc: "Square wave oscillator.",
        example: "// square wave at 110 Hz\nsq 110",
    },
    DocItem {
        symbol: "mul",
        desc: "Multiplier (gain/volume).",
        example: "// volume control\n... >> mul 0.5",
    },
    DocItem {
        symbol: "add",
        desc: "Add value.",
        example: "// Add a DC offset\n... >> add 0.5",
    },
    DocItem {
        symbol: "sp",
        desc: "Sampler.",
        example: "// Sample playback after adding sample ID 808_kick\nout: imp 1 >> sp \\808_kick >> mul 0.7",
    },
];

const STRUDEL_DOCS: &[DocItem] = &[
    DocItem {
        symbol: "note",
        desc: "Play a sequence using mini-notation. Prefix lanes with `$:`.",
        example: "// Play C Major chord sequence\n$: note(\"c3 e3 g3\")",
    },
    DocItem {
        symbol: "s",
        desc: "Set the synthesizer or sample.",
        example: "// Use the sawbass synth\n$: note(\"c3\").s(\"sawbass\")",
    },
];

const RHAI_DOCS: &[DocItem] = &[
    DocItem {
        symbol: "note",
        desc: "Set notes for a pattern.",
        example: "// Play C Major sequence\nnote(\"c3 e3 g3\")",
    },
    DocItem {
        symbol: "sound",
        desc: "Set the timbre/synth.",
        example: "// Use sawbass\nnote(\"c3\").sound(\"sawbass\")",
    },
    DocItem {
        symbol: "fast",
        desc: "Speed up the pattern.",
        example: "// Play twice as fast\nnote(\"c3 d3\").fast(2.0)",
    },
    DocItem {
        symbol: "slow",
        desc: "Slow down the pattern.",
        example: "// Play half as fast\nnote(\"c3 d3\").slow(2.0)",
    },
    DocItem {
        symbol: "rev",
        desc: "Reverse the pattern.",
        example: "// Reverse the sequence\nnote(\"c3 e3 g3\").rev()",
    },
    DocItem {
        symbol: "play",
        desc: "Start playing the pattern.",
        example: "// Compile and play\nnote(\"c3 d3\").sound(\"tb303\")",
    },
];

/// This function keeps custom sample registration on the editor side.
/// It decodes the user-selected file, persists the ID/path reference, and sends the realtime
/// engines a ready-to-register sample buffer.
fn load_and_register_sample(
    params: &Arc<GlykonsHairParams>,
    state: &mut EditorState,
    id: &str,
    path: &str,
) -> Result<(), String> {
    let path = Path::new(path);
    let loaded_sample = load_wav_sample(id, path)?;

    if let Ok(mut library) = params.user_samples.write() {
        let path_text = path.display().to_string();
        if let Some(existing) = library.samples.iter_mut().find(|sample| sample.id == id) {
            existing.path = path_text;
        } else {
            library.samples.push(UserSampleRef {
                id: id.to_string(),
                path: path_text,
            });
        }
    }

    state
        .loaded_sample_symbols
        .insert(loaded_sample.symbol.clone());
    let _ = state.tx.try_send(AudioMessage::LoadSample {
        symbol: loaded_sample.symbol,
        samples: loaded_sample.samples,
        channels: loaded_sample.channels,
        sample_rate: loaded_sample.sample_rate,
    });

    Ok(())
}

/// This function builds small context-aware insertions for the code editor.
/// The suggestions are intentionally simple and deterministic so they can act like guardrails
/// while the project grows toward richer language-aware completion.
fn suggestions_for_text(mode: SyntaxMode, text: &str) -> Vec<Suggestion> {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return match mode {
            SyntaxMode::Glicol => vec![
                Suggestion { label: "Start sine", snippet: "out: sin 220 >> mul 0.5".to_string() },
                Suggestion { label: "Start sequence", snippet: "~trig: speed 4.0 >> seq 220 _ 330 _\nout: ~trig >> saw >> lpf 1200 1.0 >> mul 0.4".to_string() },
            ],
            SyntaxMode::Strudel | SyntaxMode::Rhai => vec![
                Suggestion { label: "Notes", snippet: "note(\"c3 e3 g3\").sound(\"sawbass\")".to_string() },
                Suggestion { label: "Drums", snippet: "note(\"0 _ 0 _\").sound(\"kick\")".to_string() },
            ],
            SyntaxMode::FoxDot => vec![
                Suggestion { label: "Player", snippet: "p1 >> sawbass([0, 2, 4], dur=2)".to_string() },
            ],
        };
    }

    if trimmed.ends_with(">>") {
        return vec![
            Suggestion {
                label: "Gain",
                snippet: " mul 0.5".to_string(),
            },
            Suggestion {
                label: "Lowpass",
                snippet: " lpf 1200 1.0".to_string(),
            },
            Suggestion {
                label: "Highpass",
                snippet: " hpf 200 1.0".to_string(),
            },
        ];
    }

    if matches!(mode, SyntaxMode::Strudel | SyntaxMode::Rhai) && trimmed.contains("note(") {
        return vec![
            Suggestion {
                label: "Sawbass",
                snippet: ".sound(\"sawbass\")".to_string(),
            },
            Suggestion {
                label: "TB-303",
                snippet: ".sound(\"tb303\")".to_string(),
            },
            Suggestion {
                label: "Faster",
                snippet: ".fast(2.0)".to_string(),
            },
        ];
    }

    Vec::new()
}

/// This function inserts snippets in a way that preserves the user's current editor flow.
/// Multi-line snippets start on a new line, while chain continuations can append directly after
/// the current expression for lightweight completion behavior.
fn insert_snippet(text_buffer: &mut String, snippet: &str) {
    let append_directly = snippet.starts_with('.') || snippet.starts_with(' ');
    if !append_directly && !text_buffer.ends_with('\n') && !text_buffer.is_empty() {
        text_buffer.push('\n');
    }
    text_buffer.push_str(snippet);
    if !append_directly && !text_buffer.ends_with('\n') {
        text_buffer.push('\n');
    }
}

/// This function draws the editor's local pattern preview timeline.
/// It uses the manual BPM controls as a stand-in clock until DAW transport sync becomes part of
/// the runtime scheduler, while still showing users where their pattern repeats.
fn draw_pattern_preview(ui: &mut egui::Ui, preview: &PatternPreview, bpm: f32, running: bool) {
    let desired_size = egui::vec2(ui.available_width(), 150.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(18, 20, 22));
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 60, 65)),
        egui::StrokeKind::Outside,
    );

    if preview.events.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No visible note/seq pattern yet",
            egui::FontId::proportional(14.0),
            egui::Color32::from_rgb(150, 156, 164),
        );
        return;
    }

    let steps = preview.steps.max(1);
    for step in 0..=steps {
        let x = egui::lerp(rect.left()..=rect.right(), step as f32 / steps as f32);
        let strong = step % 4 == 0;
        let color = if strong {
            egui::Color32::from_rgb(75, 82, 90)
        } else {
            egui::Color32::from_rgb(38, 43, 48)
        };
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(if strong { 1.5 } else { 1.0 }, color),
        );
    }

    let (min_lane, max_lane) = lane_range(&preview.events);
    for event in &preview.events {
        let x = egui::lerp(
            rect.left()..=rect.right(),
            event.start_step as f32 / steps as f32,
        );
        let width = (rect.width() / steps as f32 * event.length_steps as f32 - 2.0).max(8.0);
        let y = lane_y(rect, event.lane, min_lane, max_lane, event.layer);
        let note_rect =
            egui::Rect::from_min_size(egui::pos2(x + 1.0, y - 7.0), egui::vec2(width, 14.0));
        let color = layer_color(event.layer);
        painter.rect_filled(note_rect, 2.0, color);
        painter.text(
            note_rect.left_center() + egui::vec2(3.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &event.label,
            egui::FontId::monospace(10.0),
            egui::Color32::from_rgb(15, 17, 19),
        );
    }

    if running {
        let seconds_per_step = 60.0 / bpm.max(1.0) / 4.0;
        let loop_seconds = seconds_per_step * steps as f32;
        let elapsed = ui.input(|input| input.time) as f32;
        let phase = if loop_seconds > 0.0 {
            (elapsed % loop_seconds) / loop_seconds
        } else {
            0.0
        };
        let x = egui::lerp(rect.left()..=rect.right(), phase);
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(245, 245, 245)),
        );
        ui.ctx().request_repaint();
    }
}

fn lane_range(events: &[PatternEvent]) -> (i32, i32) {
    let min_lane = events.iter().map(|event| event.lane).min().unwrap_or(48);
    let max_lane = events.iter().map(|event| event.lane).max().unwrap_or(72);
    if min_lane == max_lane {
        (min_lane - 1, max_lane + 1)
    } else {
        (min_lane, max_lane)
    }
}

fn lane_y(rect: egui::Rect, lane: i32, min_lane: i32, max_lane: i32, layer: usize) -> f32 {
    let normalized = (lane - min_lane) as f32 / (max_lane - min_lane).max(1) as f32;
    let base = egui::lerp(rect.bottom() - 18.0..=rect.top() + 18.0, normalized);
    base + (layer % 3) as f32 * 4.0
}

fn layer_color(layer: usize) -> egui::Color32 {
    const COLORS: [egui::Color32; 5] = [
        egui::Color32::from_rgb(245, 234, 35),
        egui::Color32::from_rgb(23, 220, 226),
        egui::Color32::from_rgb(232, 28, 218),
        egui::Color32::from_rgb(130, 230, 92),
        egui::Color32::from_rgb(255, 154, 46),
    ];
    COLORS[layer % COLORS.len()]
}

/// This function keeps editor state and runtime state synchronized through one consistent path.
/// It persists the text for the active MIDI note, compiles that text into runtime-ready code,
/// and only forwards successful compile results to the audio thread.
fn persist_and_dispatch_update(params: &Arc<GlykonsHairParams>, state: &mut EditorState) {
    if let Ok(mut slots) = params.pattern_slots.write() {
        slots
            .slots
            .insert(state.selected_note, state.text_buffer.clone());
    }

    match validation::compile_source_for_runtime(&state.text_buffer, state.syntax_mode) {
        Ok(compiled_code) => {
            state.last_compile_error = None;
            let _ = state.tx.try_send(AudioMessage::NewSourceText {
                note: state.selected_note,
                compiled_code,
            });
        }
        Err(error_message) => {
            state.last_compile_error = Some(error_message);
        }
    }
}

pub fn create(params: Arc<GlykonsHairParams>, tx: Sender<AudioMessage>) -> Option<Box<dyn Editor>> {
    let initial_note = 60u8;
    let initial_text = {
        if let Ok(slots) = params.pattern_slots.read() {
            slots
                .slots
                .get(&initial_note)
                .cloned()
                .unwrap_or_else(|| "out: saw 220".to_string())
        } else {
            "out: saw 220".to_string()
        }
    };

    create_egui_editor(
        params.editor_state.clone(),
        EditorState {
            selected_note: initial_note,
            text_buffer: initial_text,
            tx,
            preview_playing: false,
            search_query: String::new(),
            linked_file: None,
            last_modified: std::time::UNIX_EPOCH,
            syntax_mode: SyntaxMode::Glicol,
            last_compile_error: None,
            sample_id_buffer: String::new(),
            sample_path_buffer: String::new(),
            sample_load_error: None,
            loaded_sample_symbols: HashSet::new(),
            pattern_preview_running: true,
            pattern_preview_bpm: 120.0,
            pattern_preview_steps: 16,
        },
        |_, _| {},
        move |egui_ctx, setter, state| {
            let mut note_changed = false;
            let mut previous_note_before_change: Option<u8> = None;

            let persisted_samples = params
                .user_samples
                .read()
                .map(|library| library.samples.clone())
                .unwrap_or_default();
            for sample in persisted_samples {
                if let Ok(symbol) = sample_symbol_from_id(&sample.id) {
                    if !state.loaded_sample_symbols.contains(&symbol) {
                        let id = sample.id.clone();
                        let path = sample.path.clone();
                        if let Err(error_message) =
                            load_and_register_sample(&params, state, &id, &path)
                        {
                            state.sample_load_error =
                                Some(format!("{}: {}", sample.id, error_message));
                        }
                    }
                }
            }

            // File Watcher Polling
            if let Some(path) = &state.linked_file {
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified > state.last_modified {
                            state.last_modified = modified;
                            if let Ok(content) = std::fs::read_to_string(path) {
                                state.text_buffer = content;
                                persist_and_dispatch_update(&params, state);
                            }
                        }
                    }
                }
            }

            // RIGHT PANEL: Documentation
            egui::SidePanel::right("docs_panel")
                .resizable(true)
                .default_width(300.0)
                .show(egui_ctx, |ui| {
                    let (title, docs_list) = match state.syntax_mode {
                        SyntaxMode::Glicol => ("Glicol Reference", GLICOL_DOCS),
                        SyntaxMode::Strudel => ("Strudel Reference", STRUDEL_DOCS),
                        SyntaxMode::FoxDot => ("FoxDot Reference", RHAI_DOCS),
                        SyntaxMode::Rhai => ("Rhai Reference", RHAI_DOCS),
                    };

                    ui.heading(title);
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut state.search_query);
                    });

                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for item in docs_list.iter() {
                            if state.search_query.is_empty() || item.symbol.contains(&state.search_query) {
                                ui.group(|ui| {
                                    ui.strong(item.symbol);
                                    ui.label(item.desc);
                                    ui.horizontal(|ui| {
                                        if ui.button("Insert").clicked() {
                                            insert_snippet(&mut state.text_buffer, item.symbol);
                                            persist_and_dispatch_update(&params, state);
                                        }
                                        if ui.button("Example").clicked() {
                                            insert_snippet(&mut state.text_buffer, item.example);
                                            persist_and_dispatch_update(&params, state);
                                        }
                                    });
                                });
                            }
                        }
                    });

                    ui.separator();
                    ui.heading("Timbres");
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            for timbre in TIMBRE_NAMES {
                                if state.search_query.is_empty()
                                    || timbre.contains(&state.search_query)
                                {
                                    ui.horizontal(|ui| {
                                        ui.monospace(*timbre);
                                        if ui.button("Use").clicked() {
                                            let snippet = match state.syntax_mode {
                                                SyntaxMode::Glicol => format!(
                                                    "~p1_trig: speed 4.0 >> seq 60 _ 64 _\n~p1_pitch: ~p1_trig >> mul {}\n{}\nout: ~p1_out >> mul 0.5",
                                                    crate::translator::GLICOL_MIDDLE_C_HZ,
                                                    crate::timbres::get_timbre_patch(timbre, "p1").trim()
                                                ),
                                                SyntaxMode::FoxDot => {
                                                    format!("p1 >> {timbre}([0, 2, 4], dur=2)")
                                                }
                                                SyntaxMode::Strudel | SyntaxMode::Rhai => {
                                                    format!("note(\"c3 e3 g3\").sound(\"{timbre}\")")
                                                }
                                            };
                                            insert_snippet(&mut state.text_buffer, &snippet);
                                            persist_and_dispatch_update(&params, state);
                                        }
                                    });
                                }
                            }
                        });
                });

            // TOP PANEL: MIDI Keys
            egui::TopBottomPanel::top("header_panel").show(egui_ctx, |ui| {
                ui.heading("Glykon's Hair");
                ui.label("Select MIDI Key:");

                egui::ScrollArea::horizontal()
                    .auto_shrink([false, true]) // Do not shrink horizontally, DO shrink vertically to fit content
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for note in 36..=96 {
                                // C2 to C7
                                let is_selected = state.selected_note == note;
                                let mut button = egui::Button::new(format!("{}", note));
                                if is_selected {
                                    button = button.fill(egui::Color32::from_rgb(50, 100, 200));
                                }

                                if ui.add(button).clicked() {
                                    if state.selected_note != note {
                                        previous_note_before_change = Some(state.selected_note);
                                        state.selected_note = note;
                                        note_changed = true;
                                    }
                                }
                            }
                        });
                        // Add padding so the scrollbar renders below the buttons, not on top of them
                        ui.add_space(15.0);
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Syntax Mode:");
                    if ui
                        .radio_value(&mut state.syntax_mode, SyntaxMode::Glicol, "Glicol")
                        .changed()
                    {
                        persist_and_dispatch_update(&params, state);
                    }
                    if ui
                        .radio_value(&mut state.syntax_mode, SyntaxMode::Strudel, "Strudel")
                        .changed()
                    {
                        persist_and_dispatch_update(&params, state);
                    }
                    if ui
                        .radio_value(&mut state.syntax_mode, SyntaxMode::FoxDot, "FoxDot")
                        .changed()
                    {
                        persist_and_dispatch_update(&params, state);
                    }
                    if ui
                        .radio_value(
                            &mut state.syntax_mode,
                            SyntaxMode::Rhai,
                            "Rhai (FoxDot-like)",
                        )
                        .changed()
                    {
                        persist_and_dispatch_update(&params, state);
                    }
                });
            });

            // BOTTOM PANEL: Plugin Controls
            egui::TopBottomPanel::bottom("footer_panel")
                .resizable(false)
                .show(egui_ctx, |ui| {
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.label("Dummy Param:");
                        ui.add(widgets::ParamSlider::for_param(&params.dummy, setter));
                    });
                    ui.add_space(5.0);
                });

            // CENTRAL PANEL: Editor & Play Button
            egui::CentralPanel::default().show(egui_ctx, |ui| {
                let mode_name = match state.syntax_mode {
                    SyntaxMode::Glicol => "Glicol",
                    SyntaxMode::Strudel => "Strudel",
                    SyntaxMode::FoxDot => "FoxDot",
                    SyntaxMode::Rhai => "Rhai",
                };

                ui.horizontal(|ui| {
                    ui.heading(format!("Pattern Editor ({mode_name}) - Note {}", state.selected_note));

                    ui.add_space(20.0);

                    // Audition button
                    let play_text = if state.preview_playing { "■ STOP" } else { "▶ PLAY" };
                    let mut play_btn = egui::Button::new(play_text).min_size(egui::vec2(80.0, 30.0));
                    if state.preview_playing {
                        play_btn = play_btn.fill(egui::Color32::from_rgb(200, 50, 50));
                    } else {
                        play_btn = play_btn.fill(egui::Color32::from_rgb(50, 150, 50));
                    }

                    if ui.add(play_btn).clicked() {
                        state.preview_playing = !state.preview_playing;
                        if state.preview_playing {
                            let _ = state.tx.try_send(AudioMessage::PreviewNoteOn { note: state.selected_note });
                        } else {
                            let _ = state.tx.try_send(AudioMessage::PreviewNoteOff { note: state.selected_note });
                        }
                    }

                    ui.add_space(20.0);

                    // Panic Button
                    let panic_btn = egui::Button::new("🛑 PANIC / KILL ALL")
                        .min_size(egui::vec2(150.0, 30.0))
                        .fill(egui::Color32::from_rgb(180, 40, 40));
                    if ui.add(panic_btn).clicked() {
                        state.preview_playing = false; // Reset visual toggle
                        let _ = state.tx.try_send(AudioMessage::Panic);
                    }
                });

                ui.add_space(10.0);

                if note_changed {
                    if state.preview_playing {
                        let note_to_stop = previous_note_before_change.unwrap_or(state.selected_note);
                        let _ = state.tx.try_send(AudioMessage::PreviewNoteOff { note: note_to_stop });
                        state.preview_playing = false;
                    }

                    if let Ok(slots) = params.pattern_slots.read() {
                        state.text_buffer = slots.slots.get(&state.selected_note).cloned().unwrap_or_else(|| "out: saw 220".to_string());
                    }

                    state.last_compile_error =
                        validation::compile_source_for_runtime(&state.text_buffer, state.syntax_mode).err();
                }

                ui.horizontal(|ui| {
                    // Clear button
                    if ui.button("Clear Text").clicked() {
                        state.text_buffer.clear();
                        persist_and_dispatch_update(&params, state);
                    }

                    // Live Coding Link Button
                    if ui.button("📂 Watch ./pattern.glicol").clicked() {
                        let path = std::path::PathBuf::from("pattern.glicol");
                        if !path.exists() {
                            let _ = std::fs::write(&path, "out: saw 220 >> mul 0.5");
                        }
                        state.linked_file = Some(path);
                        state.last_modified = std::time::UNIX_EPOCH; // trigger immediate load
                    }
                    if let Some(path) = &state.linked_file {
                        ui.label(format!("Linked: {}", path.display()));
                        if ui.button("❌ Unlink").clicked() {
                            state.linked_file = None;
                        }
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Sample ID:");
                    ui.text_edit_singleline(&mut state.sample_id_buffer);
                    ui.label("WAV path:");
                    ui.text_edit_singleline(&mut state.sample_path_buffer);

                    if ui.button("Add / Reload Sample").clicked() {
                        let id = state.sample_id_buffer.trim().to_string();
                        let path = state.sample_path_buffer.trim().to_string();
                        match load_and_register_sample(&params, state, &id, &path) {
                            Ok(()) => state.sample_load_error = None,
                            Err(error_message) => state.sample_load_error = Some(error_message),
                        }
                    }
                });

                if let Some(error_message) = &state.sample_load_error {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 90, 90),
                        format!("Sample error: {error_message}"),
                    );
                }

                let samples = params
                    .user_samples
                    .read()
                    .map(|library| library.samples.clone())
                    .unwrap_or_default();
                if !samples.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Samples:");
                        for sample in samples {
                            if ui.button(&sample.id).clicked() {
                                if let Ok(symbol) = sample_symbol_from_id(&sample.id) {
                                    let snippet = format!(
                                        "~sample_trig: imp 1\nout: ~sample_trig >> sp {symbol} >> mul 0.7"
                                    );
                                    insert_snippet(&mut state.text_buffer, &snippet);
                                    persist_and_dispatch_update(&params, state);
                                }
                            }
                        }
                    });
                }

                // Editable text box
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut state.text_buffer)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    );

                    if response.changed() {
                        persist_and_dispatch_update(&params, state);
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Pattern Preview");
                    ui.checkbox(&mut state.pattern_preview_running, "Run");
                    ui.add(
                        egui::DragValue::new(&mut state.pattern_preview_bpm)
                            .range(20.0..=300.0)
                            .speed(1.0)
                            .suffix(" BPM"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut state.pattern_preview_steps)
                            .range(4..=64)
                            .speed(1.0)
                            .prefix("Steps "),
                    );
                });
                let pattern_preview = build_pattern_preview(
                    &state.text_buffer,
                    state.syntax_mode,
                    state.pattern_preview_steps,
                );
                draw_pattern_preview(
                    ui,
                    &pattern_preview,
                    state.pattern_preview_bpm,
                    state.pattern_preview_running,
                );

                let suggestions = suggestions_for_text(state.syntax_mode, &state.text_buffer);
                if !suggestions.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Suggestions:");
                        for suggestion in suggestions {
                            if ui.button(suggestion.label).clicked() {
                                insert_snippet(&mut state.text_buffer, &suggestion.snippet);
                                persist_and_dispatch_update(&params, state);
                            }
                        }
                    });
                }

                if let Some(error_message) = &state.last_compile_error {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::from_rgb(220, 90, 90), format!("Compile error: {error_message}"));
                }
            });
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // This test keeps lightweight completion useful by making sure chain continuations show
    // concrete next-step choices when the editor text ends with a Glicol pipe.
    fn suggestions_offer_chain_continuations() {
        let suggestions = suggestions_for_text(SyntaxMode::Glicol, "out: saw 220 >>");
        assert!(suggestions
            .iter()
            .any(|suggestion| suggestion.label == "Lowpass"));
        assert!(suggestions
            .iter()
            .any(|suggestion| suggestion.snippet.contains("mul")));
    }
}
