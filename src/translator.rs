use crate::timbres::get_timbre_patch;

/// This helper converts note names, scale degrees, or rests into the frequency tokens Glicol uses.
/// Strudel, FoxDot, and the Rhai facade all feed into this same runtime representation so their
/// user-facing syntaxes stay musically consistent.
pub fn note_token_to_glicol_freq(note: &str) -> String {
    let lower = note
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_lowercase();
    let midi = match lower.as_str() {
        "c2" => 48,
        "c#2" | "db2" => 49,
        "d2" => 50,
        "d#2" | "eb2" => 51,
        "e2" => 52,
        "f2" => 53,
        "f#2" | "gb2" => 54,
        "g2" => 55,
        "g#2" | "ab2" => 56,
        "a2" => 57,
        "a#2" | "bb2" => 58,
        "b2" => 59,
        "c3" => 60,
        "c#3" | "db3" => 61,
        "d3" => 62,
        "d#3" | "eb3" => 63,
        "e3" => 64,
        "f3" => 65,
        "f#3" | "gb3" => 66,
        "g3" => 67,
        "g#3" | "ab3" => 68,
        "a3" => 69,
        "a#3" | "bb3" => 70,
        "b3" => 71,
        "c4" => 72,
        "c#4" | "db4" => 73,
        "d4" => 74,
        "d#4" | "eb4" => 75,
        "e4" => 76,
        "f4" => 77,
        "f#4" | "gb4" => 78,
        "g4" => 79,
        "g#4" | "ab4" => 80,
        "a4" => 81,
        "a#4" | "bb4" => 82,
        "b4" => 83,
        "~" | "_" => return "_".to_string(),
        other => {
            if let Ok(degree) = other.parse::<i32>() {
                let scale = [60, 62, 64, 65, 67, 69, 71];
                let octave = degree.div_euclid(7);
                let note_idx = degree.rem_euclid(7) as usize;
                scale[note_idx] + (octave * 12)
            } else {
                return "_".to_string();
            }
        }
    };

    let freq = 440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0);
    format!("{freq:.2}")
}

/// This helper extracts a quoted argument from function-style text like `note("c3 e3")`.
/// The translators stay intentionally modest for now, but centralizing this keeps the prototype
/// predictable and easy to replace with a richer parser later.
fn extract_quoted_arg<'a>(input: &'a str, function_name: &str) -> Option<&'a str> {
    let double_start = format!("{function_name}(\"");
    if let Some(start) = input.find(&double_start) {
        let rest = &input[start + double_start.len()..];
        return rest.find("\")").map(|end| &rest[..end]);
    }

    let single_start = format!("{function_name}('");
    if let Some(start) = input.find(&single_start) {
        let rest = &input[start + single_start.len()..];
        return rest.find("')").map(|end| &rest[..end]);
    }

    None
}

/// This helper turns mini-notation-ish whitespace tokens into a Glicol sequence.
/// It covers the project's first live-coding grammar layer without pretending to be full Strudel.
fn translate_note_sequence(notes_str: &str) -> String {
    notes_str
        .replace('[', " ")
        .replace(']', " ")
        .replace(',', " ")
        .split_whitespace()
        .flat_map(|token| {
            let mut pieces = token.split('*');
            let note = pieces.next().unwrap_or(token);
            let repeat_count = pieces
                .next()
                .and_then(|count| count.parse::<usize>().ok())
                .unwrap_or(1);
            std::iter::repeat(note_token_to_glicol_freq(note)).take(repeat_count)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn translate_strudel(input: &str) -> String {
    let notes_str = extract_quoted_arg(input, "note").unwrap_or("0");
    let synth_name = extract_quoted_arg(input, "s")
        .or_else(|| extract_quoted_arg(input, "sound"))
        .unwrap_or("sawbass");
    let glicol_seq = translate_note_sequence(notes_str);
    let prefix = "p1";
    let timbre_patch = get_timbre_patch(synth_name, prefix);

    let mut out = String::new();
    // Default to speed 4.0 (16th notes at 60bpm or whatever the internal clock is)
    out.push_str(&format!(
        "~{prefix}_trig: speed 4.0 >> seq {}\n",
        glicol_seq.trim()
    ));
    out.push_str(&format!("~{prefix}_pitch: ~{prefix}_trig >> mul 1.0\n"));
    out.push_str(timbre_patch.trim());
    out.push('\n');
    out.push_str(&format!("out: ~{prefix}_out >> mul 0.5\n"));
    out
}

pub fn translate_foxdot(input: &str) -> String {
    let synth_name = input
        .split(">>")
        .nth(1)
        .and_then(|right| right.split('(').next())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("sawbass");

    let notes_str = input
        .find('[')
        .and_then(|start| {
            input[start + 1..]
                .find(']')
                .map(|end| &input[start + 1..start + 1 + end])
        })
        .unwrap_or("0 2 4");

    let speed = input
        .find("dur=")
        .and_then(|start| {
            input[start + 4..]
                .split(|ch: char| ch == ')' || ch == ',' || ch.is_whitespace())
                .next()
                .and_then(|value| value.parse::<f32>().ok())
        })
        .map(|dur| (4.0 / dur.max(0.25)).clamp(0.25, 16.0))
        .unwrap_or(4.0);

    let prefix = "p1";
    let timbre_patch = get_timbre_patch(synth_name, prefix);
    let glicol_seq = translate_note_sequence(notes_str);

    let mut out = String::new();
    out.push_str(&format!(
        "~{prefix}_trig: speed {speed} >> seq {glicol_seq}\n"
    ));
    out.push_str(&format!("~{prefix}_pitch: ~{prefix}_trig >> mul 1.0\n"));
    out.push_str(timbre_patch.trim());
    out.push('\n');
    out.push_str(&format!("out: ~{prefix}_out >> mul 0.5\n"));
    out
}
