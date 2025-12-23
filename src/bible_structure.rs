use std::{collections::HashMap, sync::OnceLock};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibleStructure {
    pub ot: HashMap<String, Vec<u32>>,
    pub nt: HashMap<String, Vec<u32>>,
}

const BIBLE_STRUCTURE_STR: &str = include_str!("../bible_structure.json");
static BIBLE_STRUCTURE: OnceLock<BibleStructure> = OnceLock::new();

pub fn get_bible_structure() -> &'static BibleStructure {
    BIBLE_STRUCTURE.get_or_init(|| {
        let structure: BibleStructure =
            serde_json::from_str(BIBLE_STRUCTURE_STR).expect("Failed to parse bible structure");
        structure
    })
}
