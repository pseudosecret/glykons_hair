use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets};
use std::sync::Arc;
use crossbeam::channel::Sender;

use crate::params::GlykonsHairParams;
use crate::validation::{AudioMessage, SyntaxMode};

struct EditorState {
    selected_note: u8,
    text_buffer: String,
    tx: Sender<AudioMessage>,
    preview_playing: bool,
    search_query: String,
    linked_file: Option<std::path::PathBuf>,
    last_modified: std::time::SystemTime,
    syntax_mode: SyntaxMode,
}

struct DocItem {
    symbol: &'static str,
    desc: &'static str,
    example: &'static str,
}

const GLICOL_DOCS: &[DocItem] = &[
    DocItem { symbol: "out", desc: "Master output node.", example: "out: sin 220 >> mul 0.5 // Master output node" },
    DocItem { symbol: "sin", desc: "Sine wave oscillator.", example: "// sine wave at 440 Hz\nsin 440" },
    DocItem { symbol: "saw", desc: "Sawtooth wave oscillator.", example: "// sawtooth wave at 220 Hz\nsaw 220 >> mul 0.5" },
    DocItem { symbol: "sq", desc: "Square wave oscillator.", example: "// square wave at 110 Hz\nsq 110" },
    DocItem { symbol: "mul", desc: "Multiplier (gain/volume).", example: "// volume control\n... >> mul 0.5" },
    DocItem { symbol: "add", desc: "Add value.", example: "// Add a DC offset\n... >> add 0.5" },
    DocItem { symbol: "sp", desc: "Sampler.", example: "// Sample playback\nsp \\808_kick\\" },
];

const STRUDEL_DOCS: &[DocItem] = &[
    DocItem { symbol: "note", desc: "Play a sequence of notes using mini-notation.", example: "// Play C Major chord sequence\nnote(\"c3 e3 g3\")" },
    DocItem { symbol: "s", desc: "Set the synthesizer or sample.", example: "// Use the sawbass synth\nnote(\"c3\").s(\"sawbass\")" },
];

const RHAI_DOCS: &[DocItem] = &[
    DocItem { symbol: "note", desc: "Set notes for a pattern.", example: "// Play C Major sequence\nnote(\"c3 e3 g3\")" },
    DocItem { symbol: "sound", desc: "Set the timbre/synth.", example: "// Use sawbass\nnote(\"c3\").sound(\"sawbass\")" },
    DocItem { symbol: "fast", desc: "Speed up the pattern.", example: "// Play twice as fast\nnote(\"c3 d3\").fast(2.0)" },
    DocItem { symbol: "slow", desc: "Slow down the pattern.", example: "// Play half as fast\nnote(\"c3 d3\").slow(2.0)" },
    DocItem { symbol: "rev", desc: "Reverse the pattern.", example: "// Reverse the sequence\nnote(\"c3 e3 g3\").rev()" },
    DocItem { symbol: "play", desc: "Start playing the pattern.", example: "// Compile and play\nnote(\"c3 d3\").sound(\"tb303\")" },
];

pub fn create(params: Arc<GlykonsHairParams>, tx: Sender<AudioMessage>) -> Option<Box<dyn Editor>> {
    let initial_note = 60u8;
    let initial_text = {
        if let Ok(slots) = params.pattern_slots.read() {
            slots.slots.get(&initial_note).cloned().unwrap_or_else(|| "out: saw 220".to_string())
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
        },
        |_, _| {},
        move |egui_ctx, setter, state| {
            let mut note_changed = false;

            // File Watcher Polling
            if let Some(path) = &state.linked_file {
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified > state.last_modified {
                            state.last_modified = modified;
                            if let Ok(content) = std::fs::read_to_string(path) {
                                state.text_buffer = content;
                                
                                // Push update to audio engine
                                if let Ok(mut slots) = params.pattern_slots.write() {
                                    slots.slots.insert(state.selected_note, state.text_buffer.clone());
                                }
                                let _ = state.tx.try_send(AudioMessage::NewSourceText {
                                    note: state.selected_note,
                                    valid_code: state.text_buffer.clone(),
                                    mode: state.syntax_mode,
                                });
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
                        SyntaxMode::Rhai | SyntaxMode::FoxDot => ("Rhai Reference", RHAI_DOCS),
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
                                            state.text_buffer.push_str(" ");
                                            state.text_buffer.push_str(item.symbol);
                                            state.text_buffer.push_str(" ");
                                            
                                            // Auto-save when inserted
                                            if let Ok(mut slots) = params.pattern_slots.write() {
                                                slots.slots.insert(state.selected_note, state.text_buffer.clone());
                                            }
                                            let _ = state.tx.try_send(AudioMessage::NewSourceText {
                                                note: state.selected_note,
                                                valid_code: state.text_buffer.clone(),
                                                mode: state.syntax_mode,
                                            });
                                        }
                                        if ui.button("Example").clicked() {
                                            if !state.text_buffer.ends_with('\n') && !state.text_buffer.is_empty() {
                                                state.text_buffer.push_str("\n");
                                            }
                                            state.text_buffer.push_str(item.example);
                                            state.text_buffer.push_str("\n");
                                            
                                            // Auto-save when inserted
                                            if let Ok(mut slots) = params.pattern_slots.write() {
                                                slots.slots.insert(state.selected_note, state.text_buffer.clone());
                                            }
                                            let _ = state.tx.try_send(AudioMessage::NewSourceText {
                                                note: state.selected_note,
                                                valid_code: state.text_buffer.clone(),
                                                mode: state.syntax_mode,
                                            });
                                        }
                                    });
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
                            for note in 36..=96 { // C2 to C7
                                let is_selected = state.selected_note == note;
                                let mut button = egui::Button::new(format!("{}", note));
                                if is_selected {
                                    button = button.fill(egui::Color32::from_rgb(50, 100, 200));
                                }
                                
                                if ui.add(button).clicked() {
                                    if state.selected_note != note {
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
                    if ui.radio_value(&mut state.syntax_mode, SyntaxMode::Glicol, "Glicol").changed() {
                        let _ = state.tx.try_send(AudioMessage::NewSourceText {
                            note: state.selected_note,
                            valid_code: state.text_buffer.clone(),
                            mode: state.syntax_mode,
                        });
                    }
                    if ui.radio_value(&mut state.syntax_mode, SyntaxMode::Strudel, "Strudel").changed() {
                        let _ = state.tx.try_send(AudioMessage::NewSourceText {
                            note: state.selected_note,
                            valid_code: state.text_buffer.clone(),
                            mode: state.syntax_mode,
                        });
                    }
                    if ui.radio_value(&mut state.syntax_mode, SyntaxMode::Rhai, "Rhai (FoxDot-like)").changed() {
                        let _ = state.tx.try_send(AudioMessage::NewSourceText {
                            note: state.selected_note,
                            valid_code: state.text_buffer.clone(),
                            mode: state.syntax_mode,
                        });
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
                ui.horizontal(|ui| {
                    ui.heading(format!("Pattern Editor (Glicol) - Note {}", state.selected_note));
                    
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
                        let _ = state.tx.try_send(AudioMessage::PreviewNoteOff { note: state.selected_note });
                        state.preview_playing = false;
                    }

                    if let Ok(slots) = params.pattern_slots.read() {
                        state.text_buffer = slots.slots.get(&state.selected_note).cloned().unwrap_or_else(|| "out: saw 220".to_string());
                    }
                }

                ui.horizontal(|ui| {
                    // Clear button
                    if ui.button("Clear Text").clicked() {
                        state.text_buffer.clear();
                        if let Ok(mut slots) = params.pattern_slots.write() {
                            slots.slots.insert(state.selected_note, String::new());
                        }
                        let _ = state.tx.try_send(AudioMessage::NewSourceText {
                            note: state.selected_note,
                            valid_code: String::new(),
                            mode: state.syntax_mode,
                        });
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

                // Editable text box
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut state.text_buffer)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    );
                    
                    if response.changed() {
                        if let Ok(mut slots) = params.pattern_slots.write() {
                            slots.slots.insert(state.selected_note, state.text_buffer.clone());
                        }
                        let _ = state.tx.try_send(AudioMessage::NewSourceText {
                            note: state.selected_note,
                            valid_code: state.text_buffer.clone(),
                            mode: state.syntax_mode,
                        });
                    }
                });
            });
        },
    )
}
