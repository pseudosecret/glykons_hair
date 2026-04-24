use crate::timbres::get_timbre_patch;

pub fn translate_strudel(input: &str) -> String {
    // Very rudimentary parser for `note("...")` and `s("...")`
    // Example: note("c3 d3 _ e3").s("sawbass")
    
    let mut notes_str = "60";
    let mut synth_name = "sawbass";
    
    if let Some(start) = input.find("note(\"") {
        let rest = &input[start + 6..];
        if let Some(end) = rest.find("\")") {
            notes_str = &rest[..end];
        }
    } else if let Some(start) = input.find("note('") {
        let rest = &input[start + 6..];
        if let Some(end) = rest.find("')") {
            notes_str = &rest[..end];
        }
    }
    
    if let Some(start) = input.find("s(\"") {
        let rest = &input[start + 3..];
        if let Some(end) = rest.find("\")") {
            synth_name = &rest[..end];
        }
    } else if let Some(start) = input.find("s('") {
        let rest = &input[start + 3..];
        if let Some(end) = rest.find("')") {
            synth_name = &rest[..end];
        }
    } else if let Some(start) = input.find("sound(\"") {
        let rest = &input[start + 7..];
        if let Some(end) = rest.find("\")") {
            synth_name = &rest[..end];
        }
    }
    
    // Parse notes string into MIDI values
    let notes: Vec<&str> = notes_str.split_whitespace().collect();
    let mut glicol_seq = String::new();
    for n in notes {
        let lower = n.to_lowercase();
        let midi = match lower.as_str() {
            "c3" => 60, "c#3" => 61, "d3" => 62, "d#3" => 63, "e3" => 64,
            "f3" => 65, "f#3" => 66, "g3" => 67, "g#3" => 68, "a3" => 69,
            "a#3" => 70, "b3" => 71, "c4" => 72,
            "~" | "_" => 0, // Glicol rests
            _ => 0, // fallback
        };
        if midi == 0 {
            glicol_seq.push_str("_ ");
        } else {
            let freq = 440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0);
            glicol_seq.push_str(&format!("{:.2} ", freq));
        }
    }
    
    let prefix = "p1";
    let timbre_patch = get_timbre_patch(synth_name, prefix);
    
    let mut out = String::new();
    // Default to speed 4.0 (16th notes at 60bpm or whatever the internal clock is)
    out.push_str(&format!("~{prefix}_trig: speed 4.0 >> seq {}\n", glicol_seq.trim()));
    out.push_str(&format!("~{prefix}_pitch: ~{prefix}_trig >> mul 1.0\n"));
    out.push_str(timbre_patch.trim());
    out.push('\n');
    out.push_str(&format!("out: ~{prefix}_out >> mul 0.5\n"));
    out
}

pub fn translate_foxdot(_input: &str) -> String {
    // Basic FoxDot translation prototype
    // e.g., p1 >> sawbass([0, 2, 4], dur=2)
    // For now, return a basic graph
    let prefix = "p1";
    let timbre_patch = get_timbre_patch("sawbass", prefix);
    
    let mut out = String::new();
    out.push_str(&format!("~{prefix}_trig: speed 4.0 >> seq 261.63 293.66 329.63\n"));
    out.push_str(&format!("~{prefix}_pitch: ~{prefix}_trig >> mul 1.0\n"));
    out.push_str(timbre_patch.trim());
    out.push('\n');
    out.push_str(&format!("out: ~{prefix}_out >> mul 0.5\n"));
    out
}
