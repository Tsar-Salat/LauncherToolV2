use crate::models::OperationResult;
use crate::services::{LootChances, LootScarcity};

/// Regenerate the hardcore-loot RobCo INI from the per-category scarcity sliders.
/// Call this on Play (before launch) so the current slider values take effect.
#[tauri::command]
pub fn apply_loot_scarcity(chances: LootChances) -> OperationResult {
    LootScarcity::apply(&chances)
}
