use crate::timbres::get_timbre_patch;
use crate::translator::{note_token_to_glicol_midi, GLICOL_MIDDLE_C_HZ};
use rhai::{Dynamic, Engine};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Clone)]
pub struct StrudelPattern {
    pub notes_raw: String,
    pub synth: String,
    pub speed: f32,
    pub is_rev: bool,
    pub effects: Vec<String>,
}

impl StrudelPattern {
    pub fn new() -> Self {
        Self {
            notes_raw: "60".to_string(),
            synth: "sawbass".to_string(),
            speed: 4.0,
            is_rev: false,
            effects: Vec::new(),
        }
    }
}

static PATTERN_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn strudel_note_to_midi(note: &str) -> String {
    note_token_to_glicol_midi(note)
}

// Very basic Mini-Notation flattener for prototype
// Converts "c3 d3" -> "60 62"
fn parse_mini_notation(input: &str) -> Vec<String> {
    let cleaned = input.replace("[", "").replace("]", ""); // strip brackets for now
    let mut tokens = Vec::new();
    for token in cleaned.split_whitespace() {
        if token.contains('*') {
            let parts: Vec<&str> = token.split('*').collect();
            if parts.len() == 2 {
                if let Ok(count) = parts[1].parse::<usize>() {
                    let midi = strudel_note_to_midi(parts[0]);
                    for _ in 0..count {
                        tokens.push(midi.clone());
                    }
                    continue;
                }
            }
        }
        tokens.push(strudel_note_to_midi(token));
    }
    tokens
}

pub fn evaluate_rhai(script: &str) -> Result<String, String> {
    let mut engine = Engine::new();
    PATTERN_COUNTER.store(0, Ordering::SeqCst);

    let glicol_output = Arc::new(Mutex::new(String::new()));
    let out_clone = glicol_output.clone();

    // Register the StrudelPattern type
    engine
        .register_type_with_name::<StrudelPattern>("Pattern")
        .register_fn("note", |mut p: StrudelPattern, n: &str| -> StrudelPattern {
            p.notes_raw = n.to_string();
            p
        })
        .register_fn("s", |mut p: StrudelPattern, s: &str| -> StrudelPattern {
            p.synth = s.to_string();
            p
        })
        .register_fn(
            "sound",
            |mut p: StrudelPattern, s: &str| -> StrudelPattern {
                p.synth = s.to_string();
                p
            },
        )
        .register_fn(
            "fast",
            |mut p: StrudelPattern, mult: Dynamic| -> StrudelPattern {
                if let Ok(v) = mult.as_float() {
                    p.speed *= v as f32;
                } else if let Ok(v) = mult.as_int() {
                    p.speed *= v as f32;
                }
                p
            },
        )
        .register_fn(
            "slow",
            |mut p: StrudelPattern, div: Dynamic| -> StrudelPattern {
                if let Ok(v) = div.as_float() {
                    p.speed /= v as f32;
                } else if let Ok(v) = div.as_int() {
                    p.speed /= v as f32;
                }
                p
            },
        )
        .register_fn("rev", |mut p: StrudelPattern| -> StrudelPattern {
            p.is_rev = !p.is_rev;
            p
        })
        .register_fn(
            "lpf",
            |mut p: StrudelPattern, freq: Dynamic| -> StrudelPattern {
                p.effects.push(format!("lpf {} 1.0", freq));
                p
            },
        )
        .register_fn(
            "hpf",
            |mut p: StrudelPattern, freq: Dynamic| -> StrudelPattern {
                p.effects.push(format!("hpf {} 1.0", freq));
                p
            },
        )
        .register_fn(
            "gain",
            |mut p: StrudelPattern, gain: Dynamic| -> StrudelPattern {
                p.effects.push(format!("mul {}", gain));
                p
            },
        )
        .register_fn("play", move |p: StrudelPattern| {
            let prefix = format!("p{}", PATTERN_COUNTER.fetch_add(1, Ordering::SeqCst));

            let mut seq_arr = parse_mini_notation(&p.notes_raw);
            if p.is_rev {
                seq_arr.reverse();
            }
            let seq_str = seq_arr.join(" ");

            let patch = get_timbre_patch(&p.synth, &prefix);

            let mut effects_chain = String::new();
            if !p.effects.is_empty() {
                effects_chain.push_str(" >> ");
                effects_chain.push_str(&p.effects.join(" >> "));
            }

            let mut out = String::new();
            out.push_str(&format!(
                "~{}_trig: speed {} >> seq {}\n",
                prefix, p.speed, seq_str
            ));
            out.push_str(&format!(
                "~{}_pitch: ~{}_trig >> mul {}\n",
                prefix, prefix, GLICOL_MIDDLE_C_HZ
            ));
            out.push_str(patch.trim());
            out.push('\n');
            out.push_str(&format!(
                "out: ~{}_out {} >> mul 0.5\n\n",
                prefix, effects_chain
            ));

            out_clone.lock().unwrap().push_str(&out);
        });

    // Global entry points that start a chain
    engine.register_fn("note", |n: &str| -> StrudelPattern {
        let mut p = StrudelPattern::new();
        p.notes_raw = n.to_string();
        p
    });
    engine.register_fn("s", |s: &str| -> StrudelPattern {
        let mut p = StrudelPattern::new();
        p.synth = s.to_string();
        p
    });
    engine.register_fn("sound", |s: &str| -> StrudelPattern {
        let mut p = StrudelPattern::new();
        p.synth = s.to_string();
        p
    });

    // Evaluate the script
    match engine.eval::<Dynamic>(script) {
        Ok(result) => {
            if result.is::<StrudelPattern>() {
                let p = result.cast::<StrudelPattern>();
                let prefix = format!("p{}", PATTERN_COUNTER.fetch_add(1, Ordering::SeqCst));

                let mut seq_arr = parse_mini_notation(&p.notes_raw);
                if p.is_rev {
                    seq_arr.reverse();
                }
                let seq_str = seq_arr.join(" ");

                let patch = get_timbre_patch(&p.synth, &prefix);

                let mut effects_chain = String::new();
                if !p.effects.is_empty() {
                    effects_chain.push_str(" >> ");
                    effects_chain.push_str(&p.effects.join(" >> "));
                }

                let mut out = String::new();
                out.push_str(&format!(
                    "~{}_trig: speed {} >> seq {}\n",
                    prefix, p.speed, seq_str
                ));
                out.push_str(&format!(
                    "~{}_pitch: ~{}_trig >> mul {}\n",
                    prefix, prefix, GLICOL_MIDDLE_C_HZ
                ));
                out.push_str(patch.trim());
                out.push('\n');
                out.push_str(&format!(
                    "out: ~{}_out {} >> mul 0.5\n",
                    prefix, effects_chain
                ));

                glicol_output.lock().unwrap().push_str(&out);
            }

            let final_glicol = glicol_output.lock().unwrap().clone();
            Ok(final_glicol)
        }
        Err(e) => Err(format!("Rhai Script Error: {}", e)),
    }
}
