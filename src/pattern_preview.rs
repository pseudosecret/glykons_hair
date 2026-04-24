use crate::validation::SyntaxMode;

#[derive(Clone, Debug)]
pub struct PatternPreview {
    pub events: Vec<PatternEvent>,
    pub steps: usize,
}

#[derive(Clone, Debug)]
pub struct PatternEvent {
    pub label: String,
    pub lane: i32,
    pub start_step: usize,
    pub length_steps: usize,
    pub layer: usize,
}

#[derive(Clone, Debug)]
struct StepToken {
    label: String,
    lane: i32,
}

/// This function creates a compact visual timeline model from the editor text.
/// It is intentionally independent from audio scheduling so the GUI can explain what a pattern
/// looks like now while a later DAW-sync layer can replace the local clock.
pub fn build_pattern_preview(
    source_text: &str,
    mode: SyntaxMode,
    visible_steps: usize,
) -> PatternPreview {
    let visible_steps = visible_steps.max(1);
    let mut events = match mode {
        SyntaxMode::Glicol => parse_glicol_sequences(source_text),
        SyntaxMode::Strudel | SyntaxMode::Rhai => parse_note_calls(source_text),
        SyntaxMode::FoxDot => parse_foxdot_players(source_text),
    };

    for event in &mut events {
        event.start_step %= visible_steps;
        event.length_steps = event.length_steps.max(1);
    }

    PatternPreview {
        events,
        steps: visible_steps,
    }
}

/// This parser handles the most useful Glicol preview case: `seq` chains.
/// Each discovered sequence becomes its own visual layer, with rests skipped and tokens repeated
/// across the configured preview loop.
fn parse_glicol_sequences(source_text: &str) -> Vec<PatternEvent> {
    source_text
        .lines()
        .enumerate()
        .flat_map(|(layer, line)| {
            line.find("seq")
                .map(|start| parse_step_groups(&line[start + 3..]))
                .unwrap_or_default()
                .into_iter()
                .enumerate()
                .flat_map(move |(step, group)| {
                    group.into_iter().map(move |token| PatternEvent {
                        label: token.label,
                        lane: token.lane,
                        start_step: step,
                        length_steps: 1,
                        layer,
                    })
                })
        })
        .collect()
}

/// This parser extracts Strudel/Rhai-style `note("...")` calls.
/// Separate calls or lines become separate layers so stacked patterns show up as colored rows.
fn parse_note_calls(source_text: &str) -> Vec<PatternEvent> {
    let mut events = Vec::new();
    let mut search_start = 0;
    let mut layer = 0;

    while let Some(relative_start) = source_text[search_start..].find("note(") {
        let start = search_start + relative_start;
        if let Some((notes, end)) = extract_first_quoted_arg(&source_text[start + 5..]) {
            for (step, group) in parse_step_groups(notes).into_iter().enumerate() {
                for token in group {
                    events.push(PatternEvent {
                        label: token.label,
                        lane: token.lane,
                        start_step: step,
                        length_steps: 1,
                        layer,
                    });
                }
            }
            layer += 1;
            search_start = start + 5 + end;
        } else {
            break;
        }
    }

    events
}

/// This parser handles a small FoxDot player form such as `p1 >> sawbass([0, 2, 4], dur=2)`.
/// It focuses on the bracketed note list because that is the part users need to see repeat.
fn parse_foxdot_players(source_text: &str) -> Vec<PatternEvent> {
    source_text
        .lines()
        .enumerate()
        .flat_map(|(layer, line)| {
            extract_bracket_body(line)
                .map(parse_step_groups)
                .unwrap_or_default()
                .into_iter()
                .enumerate()
                .flat_map(move |(step, group)| {
                    group.into_iter().map(move |token| PatternEvent {
                        label: token.label,
                        lane: token.lane,
                        start_step: step,
                        length_steps: 1,
                        layer,
                    })
                })
        })
        .collect()
}

/// This parser groups bracketed notes into the same timeline step.
/// For example, `c2 eb3 [g3 bb3]` becomes three steps, with `g3` and `bb3` sharing a start time.
fn parse_step_groups(input: &str) -> Vec<Vec<StepToken>> {
    let mut groups = Vec::new();
    let mut current = String::new();
    let mut bracket_depth: usize = 0;

    for ch in input.chars() {
        match ch {
            '[' => {
                flush_step(&mut groups, &mut current);
                bracket_depth += 1;
            }
            ']' => {
                flush_bracket_group(&mut groups, &mut current);
                bracket_depth = bracket_depth.saturating_sub(1);
            }
            ',' | ' ' | '\t' if bracket_depth == 0 => {
                flush_step(&mut groups, &mut current);
            }
            _ => current.push(ch),
        }
    }

    if bracket_depth > 0 {
        flush_bracket_group(&mut groups, &mut current);
    } else {
        flush_step(&mut groups, &mut current);
    }

    groups
}

fn flush_step(groups: &mut Vec<Vec<StepToken>>, current: &mut String) {
    let token = current.trim();
    if !token.is_empty() {
        if let Some(step_token) = parse_token(token) {
            groups.push(vec![step_token]);
        }
    }
    current.clear();
}

fn flush_bracket_group(groups: &mut Vec<Vec<StepToken>>, current: &mut String) {
    let tokens = current
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter_map(parse_token)
        .collect::<Vec<_>>();
    if !tokens.is_empty() {
        groups.push(tokens);
    }
    current.clear();
}

fn parse_token(token: &str) -> Option<StepToken> {
    let token = token
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('(')
        .trim_matches(')');

    if token.is_empty() || matches!(token, "_" | "~") || token.starts_with("//") {
        return None;
    }

    let token = token.split('*').next().unwrap_or(token);
    let lane = token_to_lane(token);

    Some(StepToken {
        label: token.to_string(),
        lane,
    })
}

fn token_to_lane(token: &str) -> i32 {
    if let Ok(value) = token.parse::<f32>() {
        if value <= 127.0 {
            return value.round() as i32;
        }
        return (69.0 + 12.0 * (value / 440.0).log2()).round() as i32;
    }

    note_name_to_midi(token).unwrap_or(60)
}

fn note_name_to_midi(note: &str) -> Option<i32> {
    let note = note.to_ascii_lowercase();
    let chars = note.chars().collect::<Vec<_>>();
    if chars.len() < 2 {
        return None;
    }

    let (name, octave_start) = if chars.get(1).is_some_and(|ch| *ch == '#' || *ch == 'b') {
        (&note[..2], 2)
    } else {
        (&note[..1], 1)
    };
    let octave = note[octave_start..].parse::<i32>().ok()?;
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

    Some(24 + octave * 12 + semitone)
}

fn extract_first_quoted_arg(input: &str) -> Option<(&str, usize)> {
    let quote = input.chars().find(|ch| *ch == '"' || *ch == '\'')?;
    let start = input.find(quote)? + quote.len_utf8();
    let rest = &input[start..];
    let end = rest.find(quote)?;
    Some((&rest[..end], start + end + quote.len_utf8()))
}

fn extract_bracket_body(input: &str) -> Option<&str> {
    let start = input.find('[')? + 1;
    let end = input[start..].find(']')?;
    Some(&input[start..start + end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // This test verifies that chord-like bracket groups share one timeline step, matching the
    // visual expectation from Strudel-style examples.
    fn bracket_groups_share_a_step() {
        let preview = build_pattern_preview(r#"note("c2 eb3 [g3 bb3]")"#, SyntaxMode::Strudel, 8);
        let third_step_events = preview
            .events
            .iter()
            .filter(|event| event.start_step == 2)
            .collect::<Vec<_>>();
        assert_eq!(third_step_events.len(), 2);
    }

    #[test]
    // This test keeps Glicol sequence previews useful for raw frequency patterns as well as named
    // note patterns.
    fn glicol_frequency_sequence_maps_to_lanes() {
        let preview =
            build_pattern_preview("~a: speed 4.0 >> seq 220 440 _ 880", SyntaxMode::Glicol, 8);
        assert_eq!(preview.events.len(), 3);
        assert!(preview.events.iter().any(|event| event.label == "440"));
    }
}
