use std::path::Path;
use std::process::Command;

use eframe::egui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectAction {
    OpenFolder,
    CloneRepository,
    SwitchWorkspace(String),
}

pub fn project_label(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string())
}

fn project_initials(path: &str) -> String {
    let label = project_label(path);
    let mut out = String::new();
    for segment in label
        .split(|ch: char| !(ch.is_ascii_alphanumeric()))
        .filter(|s| !s.is_empty())
    {
        if let Some(ch) = segment.chars().next() {
            out.push(ch.to_ascii_uppercase());
        }
        if out.len() >= 2 {
            break;
        }
    }

    if out.is_empty() {
        "P".to_string()
    } else {
        out
    }
}

pub fn render_dropdown(
    ui: &mut egui::Ui,
    current_workspace: &str,
    open_projects: &[String],
    recent_projects: &[String],
) -> Option<ProjectAction> {
    let mut action = None;

    egui::ComboBox::from_id_salt("project_filter_select")
        .width(220.0)
        .selected_text(project_label(current_workspace))
        .show_ui(ui, |ui| {
            ui.set_min_width(440.0);
            if ui.button("Open...").clicked() {
                action = Some(ProjectAction::OpenFolder);
            }
            if ui.button("Clone Repository...").clicked() {
                action = Some(ProjectAction::CloneRepository);
            }
            ui.separator();
            ui.strong("Open Projects");
            ui.add_space(4.0);

            for project in open_projects {
                let is_selected = project == current_workspace;
                ui.horizontal(|ui| {
                    ui.label(project_initials(project));
                    if ui.selectable_label(is_selected, project_label(project)).clicked() {
                        action = Some(ProjectAction::SwitchWorkspace(project.clone()));
                    }
                });
                ui.label(project.as_str());
                ui.add_space(4.0);
            }

            if !recent_projects.is_empty() {
                ui.separator();
                ui.strong("Recent Projects");
                ui.add_space(4.0);
                for project in recent_projects {
                    ui.horizontal(|ui| {
                        ui.label(project_initials(project));
                        if ui.selectable_label(false, project_label(project)).clicked() {
                            action = Some(ProjectAction::SwitchWorkspace(project.clone()));
                        }
                    });
                    ui.label(project.as_str());
                    ui.add_space(4.0);
                }
            }
        });

    action
}

pub fn open_folder_dialog() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript")
            .arg("-e")
            .arg("POSIX path of (choose folder with prompt \"Open Project Folder\")")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if selected.is_empty() {
            None
        } else {
            Some(selected)
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}
