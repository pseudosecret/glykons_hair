use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Serialize, Deserialize)]
pub struct PatternSlots {
    pub slots: HashMap<u8, String>,
}

impl Default for PatternSlots {
    fn default() -> Self {
        let mut slots = HashMap::new();
        for i in 0..128 {
            slots.insert(i as u8, "out: saw 220".to_string());
        }
        Self { slots }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct UserSampleLibrary {
    pub samples: Vec<UserSampleRef>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserSampleRef {
    pub id: String,
    pub path: String,
}

#[derive(Params)]
pub struct GlykonsHairParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[persist = "pattern-slots"]
    pub pattern_slots: Arc<RwLock<PatternSlots>>,

    #[persist = "user-samples"]
    pub user_samples: Arc<RwLock<UserSampleLibrary>>,

    #[id = "dummy"]
    pub dummy: FloatParam,
}

impl Default for GlykonsHairParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(1200, 800),
            pattern_slots: Arc::new(RwLock::new(PatternSlots::default())),
            user_samples: Arc::new(RwLock::new(UserSampleLibrary::default())),
            dummy: FloatParam::new("Dummy", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}
