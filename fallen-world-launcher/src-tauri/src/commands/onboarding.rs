use crate::services::onboarding::{OnboardingConfig, OnboardingService};
use crate::models::OperationResult;

#[tauri::command]
pub fn get_onboarding_config() -> OnboardingConfig {
    OnboardingService::load()
}

#[tauri::command]
pub fn save_onboarding_config(config: OnboardingConfig) -> OperationResult {
    match OnboardingService::save(&config) {
        Ok(()) => OperationResult {
            success: true,
            message: "Setup saved".to_string(),
        },
        Err(e) => OperationResult {
            success: false,
            message: e,
        },
    }
}

#[tauri::command]
pub fn is_onboarding_complete() -> bool {
    OnboardingService::is_complete()
}
