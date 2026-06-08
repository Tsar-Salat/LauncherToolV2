use crate::models::{LoadOrder, Mod};
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub struct LoadOrderValidator;

#[derive(Debug, Clone)]
struct PluginInfo {
    name: String,
    is_esm: bool,
    masters: Vec<String>,
}

impl LoadOrderValidator {
    /// Validate load order for issues including missing masters and circular dependencies
    pub fn validate_load_order(load_order: &LoadOrder) -> (bool, Vec<String>) {
        let mut issues = Vec::new();

        // Check for duplicate entries
        let mut seen = HashSet::new();
        for mod_id in &load_order.order {
            if !seen.insert(mod_id) {
                issues.push(format!("Duplicate mod in load order: {}", mod_id));
            }
        }

        // Check for missing master files
        for (mod_id, info) in &load_order.mod_info {
            for master in &info.master_files {
                let master_exists = load_order
                    .order
                    .iter()
                    .any(|m| m.to_lowercase() == master.to_lowercase());

                if !master_exists {
                    issues.push(format!(
                        "Missing master file for {}: {} (required but not in load order)",
                        mod_id, master
                    ));
                }
            }
        }

        // Check for circular dependencies
        let circular = Self::find_circular_dependencies(load_order);
        issues.extend(circular);

        // Check load order correctness (masters before dependents)
        let order_issues = Self::check_load_order_sequence(load_order);
        issues.extend(order_issues);

        (issues.is_empty(), issues)
    }

    /// Find circular dependencies in the load order
    fn find_circular_dependencies(load_order: &LoadOrder) -> Vec<String> {
        let mut circular = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for mod_id in &load_order.order {
            if !visited.contains(mod_id) {
                if Self::has_cycle(
                    mod_id,
                    load_order,
                    &mut visited,
                    &mut rec_stack,
                ) {
                    circular.push(format!("Circular dependency detected involving: {}", mod_id));
                }
            }
        }

        circular
    }

    fn has_cycle(
        mod_id: &str,
        load_order: &LoadOrder,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(mod_id.to_string());
        rec_stack.insert(mod_id.to_string());

        if let Some(info) = load_order.mod_info.get(mod_id) {
            for master in &info.master_files {
                if !visited.contains(master) {
                    if Self::has_cycle(master, load_order, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(master) {
                    return true;
                }
            }
        }

        rec_stack.remove(mod_id);
        false
    }

    /// Check if load order respects dependency ordering
    fn check_load_order_sequence(load_order: &LoadOrder) -> Vec<String> {
        let mut issues = Vec::new();

        for (idx, mod_id) in load_order.order.iter().enumerate() {
            if let Some(info) = load_order.mod_info.get(mod_id) {
                for master in &info.master_files {
                    // Find position of master in load order
                    if let Some(master_idx) = load_order.order.iter().position(|m| m == master) {
                        if master_idx > idx {
                            issues.push(format!(
                                "Load order issue: {} depends on {} but {} comes after {} in load order",
                                mod_id, master, master, mod_id
                            ));
                        }
                    }
                }
            }
        }

        issues
    }

    /// Check for file conflicts between mods (same files in multiple mods)
    pub fn check_conflicts(load_order: &LoadOrder) -> Vec<String> {
        let mut conflicts = Vec::new();
        let mut file_owners: HashMap<String, Vec<String>> = HashMap::new();

        // This is a simplified check - in a real implementation, we'd scan actual mod files
        // For now, we check for mods with overlapping dependencies or known conflict patterns

        for (mod_id, info) in &load_order.mod_info {
            for master in &info.master_files {
                file_owners
                    .entry(master.clone())
                    .or_insert_with(Vec::new)
                    .push(mod_id.clone());
            }
        }

        // Report mods that might conflict
        for (file, owners) in file_owners.iter() {
            if owners.len() > 1 {
                conflicts.push(format!(
                    "Potential conflict: Multiple mods modify {} (mods: {})",
                    file,
                    owners.join(", ")
                ));
            }
        }

        // Check for known incompatible mod combinations
        let incompatible_pairs = vec![
            ("Fallout4.esm", "FalloutNV.esm"),
            ("DLCUltimateEdition.esm", "DLCRobot.esm"),
        ];

        for (mod1, mod2) in incompatible_pairs {
            let has_mod1 = load_order.order.iter().any(|m| m.contains(mod1));
            let has_mod2 = load_order.order.iter().any(|m| m.contains(mod2));

            if has_mod1 && has_mod2 {
                conflicts.push(format!(
                    "Incompatible mods detected: {} and {} cannot be used together",
                    mod1, mod2
                ));
            }
        }

        conflicts
    }

    /// Suggest optimal load order based on dependencies (topological sort)
    pub fn suggest_order(load_order: &LoadOrder) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut order_stack: Vec<String> = Vec::new();

        for mod_id in &load_order.order {
            if !visited.contains(mod_id) {
                Self::topological_sort_dfs(
                    mod_id,
                    load_order,
                    &mut visited,
                    &mut order_stack,
                );
            }
        }

        order_stack.reverse();
        order_stack
    }

    fn topological_sort_dfs(
        mod_id: &str,
        load_order: &LoadOrder,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
    ) {
        visited.insert(mod_id.to_string());

        if let Some(info) = load_order.mod_info.get(mod_id) {
            for master in &info.master_files {
                if !visited.contains(master) && load_order.order.contains(&master.clone()) {
                    Self::topological_sort_dfs(master, load_order, visited, stack);
                }
            }
        }

        stack.push(mod_id.to_string());
    }

    /// Validate that all required files exist in mod folder
    pub fn validate_file_integrity(load_order: &LoadOrder, mod_folder: &str) -> (bool, Vec<String>) {
        let mut missing = Vec::new();

        for mod_id in &load_order.order {
            let mod_path = Path::new(mod_folder).join(mod_id);
            if !mod_path.exists() {
                missing.push(format!(
                    "Mod folder missing: {} (path: {})",
                    mod_id,
                    mod_path.display()
                ));
            } else if !mod_path.is_dir() {
                missing.push(format!(
                    "Expected mod folder but found file: {}",
                    mod_id
                ));
            }
        }

        // Check for master files referenced but not found
        for (mod_id, info) in &load_order.mod_info {
            for master in &info.master_files {
                let master_path = Path::new(mod_folder).join(master);
                if !master_path.exists() && !load_order.order.contains(master) {
                    missing.push(format!(
                        "Master file not found: {} (required by {})",
                        master, mod_id
                    ));
                }
            }
        }

        (missing.is_empty(), missing)
    }

    /// Get validation summary
    pub fn get_validation_summary(load_order: &LoadOrder) -> String {
        let (valid, issues) = Self::validate_load_order(load_order);
        let conflicts = Self::check_conflicts(load_order);

        let mut summary = String::new();
        summary.push_str(&format!("Total mods: {}\n", load_order.order.len()));

        if valid {
            summary.push_str("✓ Load order is valid\n");
        } else {
            summary.push_str(&format!("✗ {} issues found:\n", issues.len()));
            for issue in &issues {
                summary.push_str(&format!("  - {}\n", issue));
            }
        }

        if !conflicts.is_empty() {
            summary.push_str(&format!("\n⚠ {} potential conflicts:\n", conflicts.len()));
            for conflict in &conflicts {
                summary.push_str(&format!("  - {}\n", conflict));
            }
        }

        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ModInfo;

    #[test]
    fn test_validate_simple_load_order() {
        let mut mod_info = HashMap::new();
        mod_info.insert(
            "Fallout4.esm".to_string(),
            ModInfo {
                name: "Fallout 4".to_string(),
                version: "1.0".to_string(),
                has_esm: true,
                master_files: vec![],
            },
        );
        mod_info.insert(
            "MyMod.esp".to_string(),
            ModInfo {
                name: "My Mod".to_string(),
                version: "1.0".to_string(),
                has_esm: false,
                master_files: vec!["Fallout4.esm".to_string()],
            },
        );

        let load_order = LoadOrder {
            order: vec!["Fallout4.esm".to_string(), "MyMod.esp".to_string()],
            mod_info,
        };

        let (valid, issues) = LoadOrderValidator::validate_load_order(&load_order);
        assert!(valid);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_detect_missing_master() {
        let mut mod_info = HashMap::new();
        mod_info.insert(
            "MyMod.esp".to_string(),
            ModInfo {
                name: "My Mod".to_string(),
                version: "1.0".to_string(),
                has_esm: false,
                master_files: vec!["Fallout4.esm".to_string()],
            },
        );

        let load_order = LoadOrder {
            order: vec!["MyMod.esp".to_string()],
            mod_info,
        };

        let (valid, issues) = LoadOrderValidator::validate_load_order(&load_order);
        assert!(!valid);
        assert!(issues.iter().any(|i| i.contains("Missing master")));
    }

    #[test]
    fn test_detect_wrong_load_order() {
        let mut mod_info = HashMap::new();
        mod_info.insert(
            "MyMod.esp".to_string(),
            ModInfo {
                name: "My Mod".to_string(),
                version: "1.0".to_string(),
                has_esm: false,
                master_files: vec!["Fallout4.esm".to_string()],
            },
        );

        let load_order = LoadOrder {
            order: vec!["MyMod.esp".to_string(), "Fallout4.esm".to_string()],
            mod_info,
        };

        let (valid, issues) = LoadOrderValidator::validate_load_order(&load_order);
        assert!(!valid);
        assert!(issues.iter().any(|i| i.contains("Load order issue")));
    }
}
