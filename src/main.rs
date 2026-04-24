#![windows_subsystem = "windows"]

use nih_plug::prelude::*;
use glykons_hair::GlykonsHair;

fn main() {
    nih_export_standalone::<GlykonsHair>();
}
