use eframe::egui;

use crate::git_status::BranchScope;
use crate::tree_view;

pub enum BranchAction {
    SelectBranch { scope: BranchScope, name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchPanelState {
    pub scope: BranchScope,
    pub grouped: bool,
}

impl Default for BranchPanelState {
    fn default() -> Self {
        Self {
            scope: BranchScope::Local,
            grouped: false,
        }
    }
}

pub fn render_sidebar(
    ui: &mut egui::Ui,
    local_branches: &[String],
    remote_branches: &[String],
    state: &mut BranchPanelState,
    selected_local: Option<&str>,
    selected_remote: Option<&str>,
) -> Option<BranchAction> {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.scope, BranchScope::Local, "Local");
        ui.selectable_value(&mut state.scope, BranchScope::Remote, "Remote");
    });
    ui.checkbox(&mut state.grouped, "Group by prefix");
    ui.separator();

    let (branches, selected) = match state.scope {
        BranchScope::Local => (local_branches, selected_local),
        BranchScope::Remote => (remote_branches, selected_remote),
    };

    if branches.is_empty() {
        ui.label("No branches");
        return None;
    }

    if state.grouped {
        render_grouped(ui, branches, selected, state.scope)
    } else {
        render_list(ui, branches, selected, state.scope)
    }
}

fn render_list(
    ui: &mut egui::Ui,
    branches: &[String],
    selected: Option<&str>,
    scope: BranchScope,
) -> Option<BranchAction> {
    let mut action = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for branch in branches {
            let is_selected = selected.map(|s| s == branch.as_str()).unwrap_or(false);
            let response = ui.selectable_label(is_selected, branch);
            if response.clicked() {
                action = Some(BranchAction::SelectBranch {
                    scope,
                    name: branch.clone(),
                });
            }

            response.context_menu(|ui| {
                if ui.button("Select Branch").clicked() {
                    action = Some(BranchAction::SelectBranch {
                        scope,
                        name: branch.clone(),
                    });
                    ui.close();
                }
                if ui.button("Copy Branch Name").clicked() {
                    ui.ctx().copy_text(branch.clone());
                    ui.close();
                }
            });
        }
    });
    action
}

fn render_grouped(
    ui: &mut egui::Ui,
    branches: &[String],
    selected: Option<&str>,
    scope: BranchScope,
) -> Option<BranchAction> {
    let tree_items: Vec<_> = branches
        .iter()
        .map(|branch| tree_view::TreeItem {
            path: branch.as_str(),
            payload: branch,
        })
        .collect();

    let mut render_file = |ui: &mut egui::Ui, path: &str, file_name: &str, _payload: &String| {
        let is_selected = selected.map(|s| s == path).unwrap_or(false);
        let response = ui.selectable_label(is_selected, file_name);
        let action = if response.clicked() {
            Some(BranchAction::SelectBranch {
                scope,
                name: path.to_string(),
            })
        } else {
            None
        };
        (response, action)
    };

    let mut dir_context_menu = |ui: &mut egui::Ui, dir_path: &str, _dir_name: &str| {
        if ui.button("Copy Prefix").clicked() {
            ui.ctx().copy_text(dir_path.to_string());
            ui.close();
        }
        None
    };

    let mut file_context_menu = |ui: &mut egui::Ui, path: &str, _file_name: &str, _payload: &String| {
        if ui.button("Select Branch").clicked() {
            ui.close();
            return Some(BranchAction::SelectBranch {
                scope,
                name: path.to_string(),
            });
        }
        if ui.button("Copy Branch Name").clicked() {
            ui.ctx().copy_text(path.to_string());
            ui.close();
        }
        None
    };

    tree_view::render_tree(
        ui,
        &tree_items,
        &mut render_file,
        &mut dir_context_menu,
        &mut file_context_menu,
    )
}
