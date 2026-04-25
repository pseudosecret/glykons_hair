// Pre-defined Glicol patches that emulate higher-level synth sounds

pub const TIMBRE_NAMES: &[&str] = &[
    "sawbass",
    "tb303",
    "kick",
    "909bd",
    "snare",
    "909sd",
    "hat",
    "909hh",
    "cp",
    "pluck",
    "sine",
    "sawtooth",
    "square",
    "triangle",
    "white",
    "pink",
    "brown",
    "pad",
    "sub",
    "organ",
    "clave",
    "laser",
    "gm_synth_bass_1",
    "gm_synth_strings_1",
];

pub fn get_timbre_patch(name: &str, node_prefix: &str) -> String {
    match name {
        "sawbass" | "gm_synth_bass_1" | "gm_synth_bass_2" | "gm_fingered_bass" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.2
~{prefix}_saw: saw ~{prefix}_pitch
~{prefix}_flt: ~{prefix}_saw >> lpf 800 1.0
~{prefix}_out: ~{prefix}_flt >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "tb303" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.4
~{prefix}_flt_env: ~{prefix}_trig >> envperc 0.01 0.2 >> mul 3000 >> add 200
~{prefix}_saw: saw ~{prefix}_pitch
~{prefix}_flt: ~{prefix}_saw >> lpf ~{prefix}_flt_env 2.0
~{prefix}_out: ~{prefix}_flt >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "kick" | "bd" | "808bd" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.005 0.5
~{prefix}_pitch_env: ~{prefix}_trig >> envperc 0.005 0.1 >> mul 150 >> add 50
~{prefix}_osc: sin ~{prefix}_pitch_env
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "909bd" | "707bd" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.005 0.3
~{prefix}_pitch_env: ~{prefix}_trig >> envperc 0.001 0.05 >> mul 300 >> add 55
~{prefix}_osc: tri ~{prefix}_pitch_env >> mul 1.5
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "snare" | "sd" | "808sd" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.005 0.2
~{prefix}_noise: noise 42 >> lpf 3000 1.0 >> mul 0.5
~{prefix}_sine: sin 180 >> envperc 0.005 0.1
~{prefix}_out: ~{prefix}_noise >> add ~{prefix}_sine >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "909sd" | "707sd" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.005 0.25
~{prefix}_noise: noise 42 >> hpf 1000 1.0 >> mul 0.8
~{prefix}_sine: sin 220 >> envperc 0.005 0.1
~{prefix}_out: ~{prefix}_noise >> add ~{prefix}_sine >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "hat" | "hh" | "808hh" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.005 0.05
~{prefix}_noise: noise 42 >> hpf 8000 1.0
~{prefix}_out: ~{prefix}_noise >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "909hh" | "707hh" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.005 0.1
~{prefix}_noise: noise 42 >> hpf 6000 1.0 >> mul 1.2
~{prefix}_out: ~{prefix}_noise >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "cp" | "808cp" | "909cp" | "707cp" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.15
~{prefix}_noise: noise 42 >> lpf 1500 1.0 >> mul 0.8
~{prefix}_out: ~{prefix}_noise >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "pluck" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.005 0.15
~{prefix}_squ: squ ~{prefix}_pitch >> lpf 1500 1.0
~{prefix}_out: ~{prefix}_squ >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "sine" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.3
~{prefix}_osc: sin ~{prefix}_pitch
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "sawtooth" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.3
~{prefix}_osc: saw ~{prefix}_pitch
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "square" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.3
~{prefix}_osc: squ ~{prefix}_pitch
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "triangle" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.3
~{prefix}_osc: tri ~{prefix}_pitch
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "white" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.3
~{prefix}_osc: noise 42
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "pink" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.3
~{prefix}_osc: noise 42 >> lpf 2000 1.0
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "brown" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.3
~{prefix}_osc: noise 42 >> lpf 400 1.0
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "pad"
        | "gm_synth_strings_1"
        | "gm_synth_strings_2"
        | "gm_string_ensemble_1"
        | "gm_string_ensemble_2" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.5 1.0
~{prefix}_saw1: saw ~{prefix}_pitch
~{prefix}_detune: ~{prefix}_pitch >> mul 1.01
~{prefix}_saw2: saw ~{prefix}_detune
~{prefix}_osc: ~{prefix}_saw1 >> add ~{prefix}_saw2 >> mul 0.5
~{prefix}_flt: ~{prefix}_osc >> lpf 2000 1.0
~{prefix}_out: ~{prefix}_flt >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "sub" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.4
~{prefix}_sub_pitch: ~{prefix}_pitch >> mul 0.5
~{prefix}_osc: sin ~{prefix}_sub_pitch
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "organ" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.02 0.8
~{prefix}_fifth: ~{prefix}_pitch >> mul 1.5
~{prefix}_oct: ~{prefix}_pitch >> mul 2.0
~{prefix}_base: sin ~{prefix}_pitch
~{prefix}_harm: sin ~{prefix}_fifth >> mul 0.4
~{prefix}_top: sin ~{prefix}_oct >> mul 0.25
~{prefix}_mix: ~{prefix}_base >> add ~{prefix}_harm >> add ~{prefix}_top
~{prefix}_out: ~{prefix}_mix >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        "clave" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.001 0.08
~{prefix}_osc: sin 1800 >> mul ~{prefix}_env
~{prefix}_out: ~{prefix}_osc >> mul 0.8
",
            prefix = node_prefix
        ),

        "laser" => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.005 0.2
~{prefix}_pitch_env: ~{prefix}_trig >> envperc 0.005 0.18 >> mul 1200 >> add 120
~{prefix}_osc: saw ~{prefix}_pitch_env >> lpf 1800 1.0
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ),

        _ => format!(
            "
~{prefix}_env: ~{prefix}_trig >> envperc 0.01 0.3
~{prefix}_osc: sin ~{prefix}_pitch
~{prefix}_out: ~{prefix}_osc >> mul ~{prefix}_env
",
            prefix = node_prefix
        ), // default to simple sine
    }
}
