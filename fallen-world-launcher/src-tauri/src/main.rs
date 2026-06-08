#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod services;
mod models;
mod commands;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Fallen World Launcher", name)
}

fn main() {
    // Fix for Wine/Proton: WebView2 requires several sandbox/integrity bypasses under Wine.
    // --no-sandbox and --disable-gpu-sandbox prevent the sandbox process from making kernel
    // calls (including GetCurrentPackageInfo MSIX identity lookup) that Wine stubs incorrectly,
    // causing an instant crash separate from the RendererCodeIntegrity failure path.
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--disable-features=RendererCodeIntegrity --no-sandbox --disable-gpu-sandbox",
    );

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            // Game commands
            commands::game::launch_game,
            commands::game::get_game_info,
            commands::game::launch_mo2,
            commands::loot::apply_loot_scarcity,
            commands::game::open_external,
            commands::game::quit_app,
            // Dashboard / live monitor commands
            commands::monitor::get_live_stats,
            commands::monitor::get_process_status,
            commands::monitor::get_changelog_data,
            commands::monitor::get_youtube_videos,
            commands::monitor::check_anomaly_update,
            commands::monitor::mark_update_seen,
            commands::monitor::get_news_banner,
            // Linux / CLF3 / Fluorine commands
            commands::linux::clf3_status,
            commands::linux::clf3_has_api_key,
            commands::linux::clf3_set_api_key,
            commands::linux::clf3_list_gpus,
            commands::linux::clf3_check_updates,
            commands::linux::clf3_install_modlist,
            commands::linux::fetch_modlist_metadata,
            commands::linux::bootstrap_install,
            commands::linux::fluorine_open,
            // System commands
            commands::system::get_system_info,
            commands::system::check_blocking_processes,
            commands::system::detect_screen_resolution,
            commands::system::detect_gpu_vendor,
            commands::system::discover_game_paths,
            commands::system::set_game_path,
            commands::system::is_game_path_configured,
            commands::system::get_configured_game_path,
            commands::system::check_and_kill_mo2,
            commands::system::mo2_startup,
            commands::system::get_pagefile_info,
            commands::system::configure_pagefile,
            commands::system::check_msvc_installed,
            commands::system::add_antivirus_exclusion,
            // Mod commands
            commands::mods::list_mods,
            commands::mods::toggle_mod,
            commands::mods::add_user_mod,
            commands::mods::move_mod,
            // ENB manager commands
            commands::enb::get_enb_status,
            commands::enb::get_enb_config,
            commands::enb::save_enb_config,
            commands::enb::apply_default_enb,
            commands::enb::install_custom_enb,
            commands::enb::remove_custom_enb,
            commands::enb::get_enb_showcase,
            // Preset commands
            commands::presets::list_presets,
            commands::presets::install_preset,
            commands::presets::remove_preset,
            commands::presets::get_active_preset,
            commands::presets::get_preset_preview,
            // Profile commands
            commands::profiles::list_profiles,
            commands::profiles::save_profile,
            commands::profiles::delete_profile,
            commands::profiles::load_profile,
            commands::profiles::activate_profile,
            commands::profiles::open_profiles_folder,
            commands::profiles::profile_exists,
            commands::profiles::rename_profile,
            commands::profiles::get_profile_metadata,
            commands::profiles::backup_saves,
            // INI commands
            commands::ini::get_ini_config,
            commands::ini::list_ini_files,
            commands::ini::read_ini_file,
            commands::ini::save_ini_changes,
            commands::ini::apply_resolution,
            commands::ini::update_ini_value,
            commands::ini::apply_preset,
            commands::ini::backup_ini,
            commands::ini::restore_ini,
            // Update commands
            commands::updates::check_updates,
            commands::updates::update_mod,
            commands::updates::get_changelog,
            commands::updates::check_launcher_update,
            // FOMOD commands
            commands::fomod::load_fomod_config,
            commands::fomod::install_fomod_options,
            commands::fomod::get_fomod_resources,
            commands::fomod::check_fomod_available,
            commands::fomod::get_fomod_image,
            // Debug commands
            commands::debug::run_diagnostics,
            commands::debug::get_logs,
            commands::debug::get_log_file_path,
            commands::debug::clear_logs,
            commands::debug::export_logs,
            commands::debug::reveal_log_file,
            // Onboarding / first-time setup commands
            commands::onboarding::get_onboarding_config,
            commands::onboarding::save_onboarding_config,
            commands::onboarding::is_onboarding_complete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
