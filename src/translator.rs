use crate::timbres::get_timbre_patch;

pub const GLICOL_MIDDLE_C_HZ: f32 = 261.63;

/// This helper converts note names, scale degrees, or rests into the MIDI-like tokens Glicol uses.
/// Glicol's `seq` node turns MIDI 60 into the pitch ratio `1.0`, so translators keep notes as
/// MIDI values first and multiply the resulting ratio by `GLICOL_MIDDLE_C_HZ` before oscillators.
pub fn note_token_to_glicol_midi(note: &str) -> String {
    note_token_to_glicol_midi_in_scale(note, None)
}

/// This helper converts Strudel `n(...)` scale degrees into concrete MIDI notes.
/// Strudel's tonal system is much richer than this prototype, but supporting root/mode strings
/// such as `C4:minor` lets degree patterns and note-name patterns share the same Glicol backend.
fn note_token_to_glicol_midi_in_scale(note: &str, scale_name: Option<&str>) -> String {
    let lower = note
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('`')
        .to_lowercase();
    if lower == "~" || lower == "_" || lower == "-" {
        return "_".to_string();
    }

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
        other => {
            if let Ok(degree) = other.parse::<i32>() {
                scale_degree_to_midi(degree, scale_name)
            } else {
                return "_".to_string();
            }
        }
    };

    midi.to_string()
}

/// This helper maps scale degrees to MIDI notes for the subset of Strudel scale syntax we support.
/// It intentionally keeps the first implementation small: root octave plus major/minor intervals.
fn scale_degree_to_midi(degree: i32, scale_name: Option<&str>) -> i32 {
    let (root_midi, intervals) =
        parse_scale_name(scale_name).unwrap_or((60, [0, 2, 4, 5, 7, 9, 11]));
    let octave = degree.div_euclid(7);
    let note_idx = degree.rem_euclid(7) as usize;

    root_midi + intervals[note_idx] + (octave * 12)
}

/// This helper parses simple scale labels like `C4:minor`.
/// It exists so `n(...)` patterns can become notes before reaching Glicol, while unsupported scale
/// labels fall back to the current C-major behavior rather than breaking live coding.
fn parse_scale_name(scale_name: Option<&str>) -> Option<(i32, [i32; 7])> {
    let scale_name = scale_name?;
    let mut parts = scale_name.split(':');
    let root = parts.next()?.trim();
    let mode = parts.next().unwrap_or("major").trim().to_lowercase();
    let root_midi = note_name_to_midi(root)?;
    let intervals = match mode.as_str() {
        "minor" | "aeolian" => [0, 2, 3, 5, 7, 8, 10],
        _ => [0, 2, 4, 5, 7, 9, 11],
    };

    Some((root_midi, intervals))
}

/// This helper converts a note name with an octave into MIDI.
/// The translator uses it for scale roots because `C4:minor` should anchor degree `0` at C4.
fn note_name_to_midi(note: &str) -> Option<i32> {
    let note = note.trim().to_lowercase();
    let split_at = note.find(|ch: char| ch.is_ascii_digit() || ch == '-')?;
    let (name, octave_text) = note.split_at(split_at);
    let octave = octave_text.parse::<i32>().ok()?;
    let semitone = match name {
        "c" => 0,
        "c#" | "db" => 1,
        "d" => 2,
        "d#" | "eb" => 3,
        "e" => 4,
        "f" => 5,
        "f#" | "gb" => 6,
        "g" => 7,
        "g#" | "ab" => 8,
        "a" => 9,
        "a#" | "bb" => 10,
        "b" => 11,
        _ => return None,
    };

    Some((octave + 1) * 12 + semitone)
}

/// This helper extracts quoted/backtick function arguments such as `note("c3")` or `n(`0 2`)`.
/// Strudel examples commonly use backticks for multiline mini-notation, so the translator needs a
/// quote-aware scanner rather than only checking for double-quoted single-line strings.
fn extract_quoted_arg(input: &str, function_name: &str) -> Option<String> {
    let needle = format!("{function_name}(");
    let start = input.find(&needle)?;
    let rest = &input[start + needle.len()..];
    let quote = rest.chars().find(|ch| matches!(ch, '"' | '\'' | '`'))?;
    let quote_start = rest.find(quote)? + quote.len_utf8();
    let after_quote = &rest[quote_start..];
    let end = after_quote.find(quote)?;

    Some(after_quote[..end].to_string())
}

#[derive(Debug, Clone)]
enum MiniNode {
    Atom(String),
    Seq(Vec<MiniNode>),
}

/// This helper turns Strudel mini-notation into a Glicol `seq` string while preserving relative
/// rhythm. Grouped patterns like `9 [1 1]` become expanded onset/rest lanes (`9 _ 1 1`) so Glicol
/// can represent quarter/eighth timing instead of flattening everything into equal slices.
fn translate_note_sequence(notes_str: &str, scale_name: Option<&str>) -> Result<String, String> {
    let ast = parse_mini_notation(notes_str)?;
    if ast.is_empty() {
        return Ok("_".to_string());
    }

    let total_steps = required_steps_for_seq(&ast).max(1);
    let mut timeline = vec!["_".to_string(); total_steps];
    render_sequence_to_timeline(&ast, 0, total_steps, &mut timeline);

    let translated = timeline
        .iter()
        .map(|token| note_token_to_glicol_midi_in_scale(token, scale_name))
        .collect::<Vec<_>>();
    Ok(translated.join(" "))
}

/// This parser builds a tiny AST for the subset of Strudel mini-notation we support.
/// Parsing into nested sequences lets us preserve subgroup durations before converting to Glicol.
fn parse_mini_notation(input: &str) -> Result<Vec<MiniNode>, String> {
    let chars = input.chars().collect::<Vec<_>>();
    let mut index = 0;
    let nodes = parse_mini_sequence(&chars, &mut index, None)?;
    if index < chars.len() {
        return Err("Unexpected trailing Strudel mini-notation tokens".to_string());
    }
    Ok(nodes)
}

/// This parser consumes one sequence level, optionally terminated by `]` or `>`.
fn parse_mini_sequence(
    chars: &[char],
    index: &mut usize,
    terminator: Option<char>,
) -> Result<Vec<MiniNode>, String> {
    let mut nodes = Vec::new();

    while *index < chars.len() {
        let ch = chars[*index];
        if Some(ch) == terminator {
            *index += 1;
            return Ok(nodes);
        }

        match ch {
            '[' => {
                *index += 1;
                let group = parse_mini_sequence(chars, index, Some(']'))?;
                let repeat = parse_repeat_count(chars, index)?;
                if repeat == 1 {
                    nodes.push(MiniNode::Seq(group));
                } else {
                    nodes.push(MiniNode::Seq(
                        (0..repeat).map(|_| MiniNode::Seq(group.clone())).collect(),
                    ));
                }
            }
            '<' => {
                *index += 1;
                let group = parse_mini_sequence(chars, index, Some('>'))?;
                let repeat = parse_repeat_count(chars, index)?;
                if repeat == 1 {
                    nodes.push(MiniNode::Seq(group));
                } else {
                    nodes.push(MiniNode::Seq(
                        (0..repeat).map(|_| MiniNode::Seq(group.clone())).collect(),
                    ));
                }
            }
            ']' | '>' => {
                return Err(format!(
                    "Unmatched Strudel group terminator `{ch}` in mini-notation"
                ));
            }
            ',' | '\n' | '\r' | '\t' | ' ' => {
                *index += 1;
            }
            _ => {
                let token = parse_token(chars, index);
                if !token.is_empty() {
                    let repeat = parse_repeat_count(chars, index)?;
                    if repeat == 1 {
                        nodes.push(MiniNode::Atom(token));
                    } else {
                        nodes.push(MiniNode::Seq(
                            (0..repeat).map(|_| MiniNode::Atom(token.clone())).collect(),
                        ));
                    }
                }
            }
        }
    }

    if let Some(expected) = terminator {
        return Err(format!(
            "Missing closing `{expected}` in Strudel mini-notation"
        ));
    }

    Ok(nodes)
}

/// This parser reads one mini-notation atom and strips suffixes that this prototype cannot time yet.
/// Weighting (`@`), probability (`?`), and division (`/`) become no-ops rather than syntax poison.
fn parse_token(chars: &[char], index: &mut usize) -> String {
    let start = *index;
    while *index < chars.len()
        && !matches!(
            chars[*index],
            '[' | ']' | '<' | '>' | ',' | '\n' | '\r' | '\t' | ' ' | '*'
        )
    {
        *index += 1;
    }

    chars[start..*index]
        .iter()
        .collect::<String>()
        .split(['@', '?', '/'])
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// This parser reads a simple integer repeat suffix like `*4`.
/// Decimal repeats are rounded down because Glicol sequences need concrete event counts.
fn parse_repeat_count(chars: &[char], index: &mut usize) -> Result<usize, String> {
    while *index < chars.len() && chars[*index].is_whitespace() {
        *index += 1;
    }

    if *index >= chars.len() || chars[*index] != '*' {
        return Ok(1);
    }

    *index += 1;
    let start = *index;
    while *index < chars.len() && (chars[*index].is_ascii_digit() || chars[*index] == '.') {
        *index += 1;
    }

    if start == *index {
        return Err("Strudel repeat operator `*` must be followed by a number".to_string());
    }

    let repeat = chars[start..*index]
        .iter()
        .collect::<String>()
        .parse::<f32>()
        .ok()
        .map(|value| value.floor().max(1.0) as usize)
        .unwrap_or(1);
    Ok(repeat)
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}

fn lcm(a: usize, b: usize) -> usize {
    (a / gcd(a, b)).saturating_mul(b).max(1)
}

fn required_steps_for_node(node: &MiniNode) -> usize {
    match node {
        MiniNode::Atom(_) => 1,
        MiniNode::Seq(nodes) => required_steps_for_seq(nodes),
    }
}

fn required_steps_for_seq(nodes: &[MiniNode]) -> usize {
    if nodes.is_empty() {
        return 1;
    }
    let mut child_lcm = 1;
    for node in nodes {
        child_lcm = lcm(child_lcm, required_steps_for_node(node));
    }

    nodes.len().saturating_mul(child_lcm).max(1)
}

fn render_sequence_to_timeline(
    nodes: &[MiniNode],
    start: usize,
    steps: usize,
    timeline: &mut [String],
) {
    if nodes.is_empty() || steps == 0 {
        return;
    }

    let child_steps = (steps / nodes.len()).max(1);
    for (idx, node) in nodes.iter().enumerate() {
        let child_start = start + idx * child_steps;
        let max_steps = timeline.len().saturating_sub(child_start);
        let bounded_steps = child_steps.min(max_steps);
        render_node_to_timeline(node, child_start, bounded_steps, timeline);
    }
}

fn render_node_to_timeline(node: &MiniNode, start: usize, steps: usize, timeline: &mut [String]) {
    if steps == 0 || start >= timeline.len() {
        return;
    }

    match node {
        MiniNode::Atom(token) => {
            timeline[start] = token.clone();
            for step in (start + 1)..(start + steps).min(timeline.len()) {
                timeline[step] = "_".to_string();
            }
        }
        MiniNode::Seq(children) => render_sequence_to_timeline(children, start, steps, timeline),
    }
}

fn find_function_call_span(input: &str, function_name: &str) -> Option<(usize, usize)> {
    let needle = format!("{function_name}(");
    let start = input.find(&needle)?;
    let open = start + function_name.len();
    let bytes = input.as_bytes();
    let mut index = open + 1;
    let mut depth = 1_i32;
    let mut quote: Option<u8> = None;

    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if byte == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        match byte {
            b'"' | b'\'' | b'`' => quote = Some(byte),
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, index + 1));
                }
            }
            _ => {}
        }

        index += 1;
    }

    None
}

fn validate_statement_syntax(statement: &str) -> Result<(), String> {
    for function_name in ["note", "n"] {
        if let Some((_, end)) = find_function_call_span(statement, function_name) {
            let after_call = statement[end..].trim_start();
            if after_call.starts_with('*') {
                return Err(format!(
                    "Invalid Strudel syntax: `*` repeats must be inside `{function_name}(...)` quotes/backticks. Example: `{function_name}(\"<c2>*3\")`."
                ));
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct StrudelPatternSpec {
    notation: String,
    synth_name: String,
    scale_name: Option<String>,
    lpf_cutoff: Option<f32>,
    speed: f32,
}

/// This helper extracts each `$:` Strudel line into a small renderable pattern spec.
/// Multiple specs become parallel Glicol chains, which is the key behavior needed for sketches with
/// bass, strings, drums, and other independent pattern lanes running at once.
fn collect_strudel_patterns(input: &str) -> Result<Vec<StrudelPatternSpec>, String> {
    let pattern_call_count = input.matches("note(").count() + input.matches("n(").count();
    if pattern_call_count > 1 && !input.contains("$:") {
        return Err(
            "Multiple Strudel patterns detected. Prefix each parallel lane with `$:`.".to_string(),
        );
    }

    let mut patterns = input
        .split("$:")
        .filter_map(|statement| parse_strudel_pattern_statement(statement).transpose())
        .collect::<Result<Vec<_>, String>>()?;

    if patterns.is_empty() {
        if let Some(pattern) = parse_strudel_pattern_statement(input)? {
            patterns.push(pattern);
        }
    }

    if patterns.is_empty() {
        return Err("No Strudel pattern found. Use `note(...)` or `n(...)`.".to_string());
    }

    Ok(patterns)
}

/// This helper parses one Strudel-ish statement.
/// It supports `note(...)`, `n(...)`, `.sound(...)`/`.s(...)`, `.scale(...)`, `.lpf(...)`,
/// and speed modifiers `.fast(...)` / `.slow(...)`.
fn parse_strudel_pattern_statement(statement: &str) -> Result<Option<StrudelPatternSpec>, String> {
    validate_statement_syntax(statement)?;
    let notation = match extract_quoted_arg(statement, "note")
        .or_else(|| extract_quoted_arg(statement, "n"))
    {
        Some(value) => value,
        None => return Ok(None),
    };
    let synth_name = extract_quoted_arg(statement, "sound")
        .or_else(|| extract_quoted_arg(statement, "s"))
        .unwrap_or_else(|| "sawbass".to_string());
    let scale_name = extract_quoted_arg(statement, "scale");
    let lpf_cutoff = extract_number_arg(statement, "lpf");
    let fast = extract_number_arg(statement, "fast").unwrap_or(1.0);
    let slow = extract_number_arg(statement, "slow").unwrap_or(1.0);
    let speed = (fast / slow.max(0.0001)).clamp(0.125, 16.0);

    Ok(Some(StrudelPatternSpec {
        notation,
        synth_name,
        scale_name,
        lpf_cutoff,
        speed,
    }))
}

/// This helper extracts numeric effect arguments such as `.lpf(800)`.
/// Effects are rendered per-pattern so a bass filter does not accidentally darken the whole mix.
fn extract_number_arg(input: &str, function_name: &str) -> Option<f32> {
    let needle = format!("{function_name}(");
    let start = input.find(&needle)?;
    let rest = &input[start + needle.len()..];
    let end = rest.find(')')?;

    rest[..end].trim().parse::<f32>().ok()
}

/// This helper renders one pattern lane into Glicol source and returns its final mix reference.
/// Keeping pattern output references explicit makes summing several `$:` lines straightforward.
fn render_strudel_pattern(
    spec: &StrudelPatternSpec,
    prefix: &str,
) -> Result<(String, String), String> {
    let glicol_seq = translate_note_sequence(&spec.notation, spec.scale_name.as_deref())?;
    let timbre_patch = get_timbre_patch(&spec.synth_name, prefix);
    let final_ref = format!("~{prefix}_final");

    let mut out = String::new();
    out.push_str(&format!(
        "~{prefix}_trig: speed {} >> seq {glicol_seq}\n",
        spec.speed
    ));
    out.push_str(&format!(
        "~{prefix}_pitch: ~{prefix}_trig >> mul {GLICOL_MIDDLE_C_HZ}\n"
    ));
    out.push_str(timbre_patch.trim());
    out.push('\n');

    if let Some(cutoff) = spec.lpf_cutoff {
        out.push_str(&format!("{final_ref}: ~{prefix}_out >> lpf {cutoff} 1.0\n"));
    } else {
        out.push_str(&format!("{final_ref}: ~{prefix}_out >> mul 1.0\n"));
    }

    Ok((out, final_ref))
}

pub fn translate_strudel(input: &str) -> Result<String, String> {
    let patterns = collect_strudel_patterns(input)?;
    let mut out = String::new();
    let mut mix_refs = Vec::new();

    for (idx, pattern) in patterns.iter().enumerate() {
        let prefix = format!("p{}", idx + 1);
        let (pattern_code, final_ref) = render_strudel_pattern(pattern, &prefix)?;
        out.push_str(&pattern_code);
        out.push('\n');
        mix_refs.push(final_ref);
    }

    let mut mix_chain = mix_refs[0].clone();
    for mix_ref in mix_refs.iter().skip(1) {
        mix_chain.push_str(&format!(" >> add {mix_ref}"));
    }
    out.push_str(&format!("out: {mix_chain} >> mul 0.5\n"));

    Ok(out)
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
        .unwrap_or(1.0);

    let prefix = "p1";
    let timbre_patch = get_timbre_patch(synth_name, prefix);
    let glicol_seq = translate_note_sequence(notes_str, None).unwrap_or_else(|_| "_".to_string());

    let mut out = String::new();
    out.push_str(&format!(
        "~{prefix}_trig: speed {speed} >> seq {glicol_seq}\n"
    ));
    out.push_str(&format!(
        "~{prefix}_pitch: ~{prefix}_trig >> mul {GLICOL_MIDDLE_C_HZ}\n"
    ));
    out.push_str(timbre_patch.trim());
    out.push('\n');
    out.push_str(&format!("out: ~{prefix}_out >> mul 0.5\n"));
    out
}
