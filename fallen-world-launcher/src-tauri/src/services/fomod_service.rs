use serde::{Deserialize, Serialize};
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GroupType {
    #[serde(rename = "SelectExactlyOne")]
    SelectExactlyOne,
    #[serde(rename = "SelectAny")]
    SelectAny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSpec {
    pub source: String,
    pub destination: String,
    pub file_type: String, // "file" or "folder"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub files: Vec<FileSpec>,
    /// Relative path (from the FOMOD root) to this plugin's preview image, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub group_type: GroupType,
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallStep {
    pub name: String,
    pub groups: Vec<Group>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_type: Option<String>, // "dependencies", "pagefile", etc
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub step_index: usize,
    pub group_index: usize,
    pub plugin_indices: Vec<usize>, // Single item for SelectExactlyOne
}

pub struct FomodService;

impl FomodService {
    pub fn parse_config(config_path: &Path) -> Result<Vec<InstallStep>, String> {
        if !config_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        Self::parse_xml(&content)
    }

    fn parse_xml(xml_content: &str) -> Result<Vec<InstallStep>, String> {
        let parser = EventReader::new(xml_content.as_bytes());
        let mut steps = Vec::new();
        let mut current_step: Option<InstallStep> = None;
        let mut current_group: Option<Group> = None;
        let mut current_plugin: Option<Plugin> = None;
        let mut text_content = String::new();

        for event in parser {
            match event.map_err(|e| format!("XML parse error: {}", e))? {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    text_content.clear();

                    match name.local_name.as_str() {
                        "installStep" => {
                            let step_name = attributes
                                .iter()
                                .find(|a| a.name.local_name == "name")
                                .map(|a| a.value.clone())
                                .unwrap_or_default();

                            let name_lower = step_name.to_lowercase();
                            // Skip GPU, Resolution, and other auto-detected steps
                            if !Self::should_skip_step(&name_lower) {
                                current_step = Some(InstallStep {
                                    name: step_name,
                                    groups: Vec::new(),
                                    special_type: None,
                                });
                            }
                        }
                        "group" => {
                            if current_step.is_some() {
                                let group_name = attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "name")
                                    .map(|a| a.value.clone())
                                    .unwrap_or_default();

                                let group_type_str = attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "type")
                                    .map(|a| a.value.clone())
                                    .unwrap_or_else(|| "SelectExactlyOne".to_string());

                                let group_type = match group_type_str.as_str() {
                                    "SelectAny" => GroupType::SelectAny,
                                    _ => GroupType::SelectExactlyOne,
                                };

                                current_group = Some(Group {
                                    name: group_name,
                                    group_type,
                                    plugins: Vec::new(),
                                });
                            }
                        }
                        "plugin" => {
                            if current_group.is_some() {
                                let plugin_name = attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "name")
                                    .map(|a| a.value.clone())
                                    .unwrap_or_default();

                                current_plugin = Some(Plugin {
                                    name: plugin_name,
                                    description: String::new(),
                                    files: Vec::new(),
                                    image: None,
                                });
                            }
                        }
                        "image" => {
                            // Per-plugin preview image: <image path="fomod/images/x.png"/>
                            if let Some(ref mut plugin) = current_plugin {
                                if let Some(path) = attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "path")
                                    .map(|a| a.value.clone())
                                {
                                    if !path.is_empty() {
                                        plugin.image = Some(path);
                                    }
                                }
                            }
                        }
                        "description" => {
                            // Will be filled on text content
                        }
                        "folder" => {
                            if let Some(ref mut plugin) = current_plugin {
                                let source = attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "source")
                                    .map(|a| a.value.clone())
                                    .unwrap_or_default();

                                let destination = attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "destination")
                                    .map(|a| a.value.clone())
                                    .unwrap_or_default();

                                if !source.is_empty() {
                                    plugin.files.push(FileSpec {
                                        source,
                                        destination,
                                        file_type: "folder".to_string(),
                                    });
                                }
                            }
                        }
                        "file" => {
                            if let Some(ref mut plugin) = current_plugin {
                                let source = attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "source")
                                    .map(|a| a.value.clone())
                                    .unwrap_or_default();

                                let destination = attributes
                                    .iter()
                                    .find(|a| a.name.local_name == "destination")
                                    .map(|a| a.value.clone())
                                    .unwrap_or_default();

                                if !source.is_empty() {
                                    plugin.files.push(FileSpec {
                                        source,
                                        destination,
                                        file_type: "file".to_string(),
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
                XmlEvent::Characters(text) => {
                    text_content.push_str(&text);
                }
                XmlEvent::EndElement { name } => {
                    match name.local_name.as_str() {
                        "description" => {
                            if let Some(ref mut plugin) = current_plugin {
                                plugin.description = text_content.trim().to_string();
                            }
                        }
                        "plugin" => {
                            if let Some(plugin) = current_plugin.take() {
                                if let Some(ref mut group) = current_group {
                                    group.plugins.push(plugin);
                                }
                            }
                        }
                        "group" => {
                            if let Some(group) = current_group.take() {
                                if !group.plugins.is_empty() {
                                    if let Some(ref mut step) = current_step {
                                        step.groups.push(group);
                                    }
                                }
                            }
                        }
                        "installStep" => {
                            if let Some(step) = current_step.take() {
                                if !step.groups.is_empty() {
                                    steps.push(step);
                                }
                            }
                        }
                        _ => {}
                    }
                    text_content.clear();
                }
                _ => {}
            }
        }

        // Add special steps
        Self::inject_special_steps(&mut steps);

        Ok(steps)
    }

    fn should_skip_step(name_lower: &str) -> bool {
        let skip_keywords = [
            "gpu",
            "graphics",
            "resolution",
            "screen",
            "video card",
            "welcome",
        ];
        skip_keywords.iter().any(|&kw| name_lower.contains(kw))
    }

    fn inject_special_steps(_steps: &mut Vec<InstallStep>) {
        // Intentionally a no-op. Display resolution, GPU vendor, system
        // prerequisites and pagefile setup belong to the first-time setup
        // flow (see services::onboarding), not the Optional Mods wizard.
        // FOMOD is now strictly about the user's mod choices parsed from
        // ModuleConfig.xml.
    }

    /// Returns wizard steps (excludes Welcome/Finalization)
    pub fn get_wizard_steps(all_steps: &[InstallStep]) -> Vec<(usize, &InstallStep)> {
        all_steps
            .iter()
            .enumerate()
            .filter(|(_, step)| {
                let name_lower = step.name.to_lowercase();
                !name_lower.contains("welcome")
                    && !name_lower.contains("final")
                    && !name_lower.contains("installation complete")
            })
            .collect()
    }

    /// Validates selection indices are within bounds
    pub fn validate_selections(
        steps: &[InstallStep],
        selections: &[Selection],
    ) -> Result<(), String> {
        for sel in selections {
            if sel.step_index >= steps.len() {
                return Err(format!("Invalid step index: {}", sel.step_index));
            }

            let step = &steps[sel.step_index];
            if sel.group_index >= step.groups.len() {
                return Err(format!(
                    "Invalid group index in step {}: {}",
                    sel.step_index, sel.group_index
                ));
            }

            let group = &step.groups[sel.group_index];
            for &plugin_idx in &sel.plugin_indices {
                if plugin_idx >= group.plugins.len() {
                    return Err(format!(
                        "Invalid plugin index in step {}, group {}: {}",
                        sel.step_index, sel.group_index, plugin_idx
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_basic() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<config xmlns="http://qconsulting.ca/fo3/ModConfig5.0.xsd">
  <moduleConfig>
    <installSteps>
      <installStep name="Graphics">
        <optionalFileGroups>
          <group name="Resolution" type="SelectExactlyOne">
            <plugins>
              <plugin name="1920x1080">
                <description>Standard HD</description>
              </plugin>
            </plugins>
          </group>
        </optionalFileGroups>
      </installStep>
    </installSteps>
  </moduleConfig>
</config>"#;

        let steps = FomodService::parse_xml(xml);
        assert!(steps.is_ok());
    }
}
