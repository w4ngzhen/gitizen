use eframe::egui;

use crate::git_status::ChangeItem;
use crate::tree_view;

pub enum DiffAction {
    SelectPath(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffDisplayMode {
    Tree,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSortKey {
    Path,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffPanelState {
    pub mode: DiffDisplayMode,
    pub sort_key: DiffSortKey,
    pub sort_order: SortOrder,
}

impl Default for DiffPanelState {
    fn default() -> Self {
        Self {
            mode: DiffDisplayMode::Tree,
            sort_key: DiffSortKey::Path,
            sort_order: SortOrder::Asc,
        }
    }
}

pub fn render_sidebar(
    ui: &mut egui::Ui,
    changes: &[ChangeItem],
    state: &mut DiffPanelState,
    selected_path: Option<&str>,
) -> Option<DiffAction> {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.mode, DiffDisplayMode::Tree, "Tree");
        ui.selectable_value(&mut state.mode, DiffDisplayMode::List, "List");
    });
    ui.separator();

    match state.mode {
        DiffDisplayMode::Tree => render_tree(ui, changes, selected_path),
        DiffDisplayMode::List => render_list(ui, changes, state, selected_path),
    }
}

fn render_list(
    ui: &mut egui::Ui,
    changes: &[ChangeItem],
    state: &mut DiffPanelState,
    selected_path: Option<&str>,
) -> Option<DiffAction> {
    ui.horizontal(|ui| {
        ui.label("Sort by:");
        egui::ComboBox::from_id_salt("diff_sort_key")
            .selected_text(match state.sort_key {
                DiffSortKey::Path => "Path",
                DiffSortKey::Status => "Status",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.sort_key, DiffSortKey::Path, "Path");
                ui.selectable_value(&mut state.sort_key, DiffSortKey::Status, "Status");
            });

        let order_label = match state.sort_order {
            SortOrder::Asc => "Asc",
            SortOrder::Desc => "Desc",
        };
        if ui.button(order_label).clicked() {
            state.sort_order = match state.sort_order {
                SortOrder::Asc => SortOrder::Desc,
                SortOrder::Desc => SortOrder::Asc,
            };
        }
    });
    ui.separator();

    let mut rows: Vec<&ChangeItem> = changes.iter().collect();
    rows.sort_by(|a, b| match state.sort_key {
        DiffSortKey::Path => a.path.cmp(&b.path).then_with(|| a.code.cmp(&b.code)),
        DiffSortKey::Status => a.code.cmp(&b.code).then_with(|| a.path.cmp(&b.path)),
    });
    if state.sort_order == SortOrder::Desc {
        rows.reverse();
    }

    let mut action = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        for item in rows {
            let label = format!("{} {}", item.code, item.path);
            let is_selected = selected_path.map(|p| p == item.path).unwrap_or(false);
            let response = ui.selectable_label(is_selected, egui::RichText::new(label).monospace());
            if response.clicked() {
                action = Some(DiffAction::SelectPath(item.path.clone()));
            }
            response.context_menu(|ui| {
                if ui.button("Select File").clicked() {
                    action = Some(DiffAction::SelectPath(item.path.clone()));
                    ui.close();
                }
                if ui.button("Copy Path").clicked() {
                    ui.ctx().copy_text(item.path.clone());
                    ui.close();
                }
                if ui.button("Copy Status").clicked() {
                    ui.ctx().copy_text(item.code.clone());
                    ui.close();
                }
            });
        }
    });
    action
}

fn render_tree(ui: &mut egui::Ui, changes: &[ChangeItem], selected_path: Option<&str>) -> Option<DiffAction> {
    let tree_items: Vec<_> = changes
        .iter()
        .map(|change| tree_view::TreeItem {
            path: change.path.as_str(),
            payload: change,
        })
        .collect();

    let mut render_file = |ui: &mut egui::Ui, path: &str, file_name: &str, item: &ChangeItem| {
            let label = format!("{} {}", item.code, file_name);
            let is_selected = selected_path.map(|p| p == path).unwrap_or(false);
            let response = ui.selectable_label(is_selected, egui::RichText::new(label).monospace());
            let action = if response.clicked() {
                Some(DiffAction::SelectPath(path.to_string()))
            } else {
                None
            };
            (response, action)
        };

    let mut dir_context_menu = |ui: &mut egui::Ui, dir_path: &str, _dir_name: &str| {
        if ui.button("Copy Directory Path").clicked() {
            ui.ctx().copy_text(dir_path.to_string());
            ui.close();
        }
        None::<DiffAction>
    };

    let mut file_context_menu =
        |ui: &mut egui::Ui, path: &str, _file_name: &str, item: &ChangeItem| {
            if ui.button("Select File").clicked() {
                ui.close();
                return Some(DiffAction::SelectPath(path.to_string()));
            }
            if ui.button("Copy Path").clicked() {
                ui.ctx().copy_text(path.to_string());
                ui.close();
            }
            if ui.button("Copy Status").clicked() {
                ui.ctx().copy_text(item.code.clone());
                ui.close();
            }
            None::<DiffAction>
        };

    tree_view::render_tree(
        ui,
        &tree_items,
        &mut render_file,
        &mut dir_context_menu,
        &mut file_context_menu,
    )
}
