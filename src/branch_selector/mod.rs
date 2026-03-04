use eframe::egui;

#[derive(Debug, Clone, Default)]
pub struct BranchSelectorState {
    pub search_query: String,
    pub show_new_branch_dialog: bool,
    pub new_branch_name: String,
    pub show_checkout_dialog: bool,
    pub checkout_reference: String,
    pub message: Option<String>,
    pub focused_local_branch: Option<String>,
    pub focused_remote_branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchSelectorAction {
    CreateBranch { name: String },
    CheckoutReference { reference: String },
    CheckoutLocalBranch { name: String },
    CheckoutRemoteBranch { name: String },
}

fn fuzzy_match(text: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let text = text.to_ascii_lowercase();
    let query = query.to_ascii_lowercase();
    text.contains(&query)
}

pub fn render_dropdown(
    ui: &mut egui::Ui,
    state: &mut BranchSelectorState,
    selected_local_branch: Option<&str>,
    local_branches: &[String],
    remote_branches: &[String],
) -> Option<BranchSelectorAction> {
    let mut action = None;
    let current_local = selected_local_branch
        .map(ToOwned::to_owned)
        .or_else(|| local_branches.first().cloned())
        .unwrap_or_else(|| "No local branch".to_string());

    egui::ComboBox::from_id_salt("branch_select")
        .width(200.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .selected_text(current_local)
        .show_ui(ui, |ui| {
            ui.set_min_width(420.0);

            if let Some(message) = &state.message {
                ui.label(message);
                ui.separator();
            }

            // Module 1: search
            ui.add_sized(
                [360.0, 30.0],
                egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text("Search branches (fuzzy)"),
            );

            draw_module_divider(ui);

            // Module 3: new branch / checkout tag or revision
            if ui.button("New Branch").clicked() {
                state.show_new_branch_dialog = true;
            }
            if ui.button("Checkout Tag or Revision").clicked() {
                state.show_checkout_dialog = true;
            }

            draw_module_divider(ui);

            // Module 4: tree-like branch list
            let query = state.search_query.trim();

            let filtered_local: Vec<_> = local_branches
                .iter()
                .filter(|name| fuzzy_match(name, query))
                .cloned()
                .collect();
            let filtered_remote: Vec<_> = remote_branches
                .iter()
                .filter(|name| fuzzy_match(name, query))
                .cloned()
                .collect();

            ui.collapsing("Local", |ui| {
                if filtered_local.is_empty() {
                    ui.label("No local branches");
                } else {
                    for branch in &filtered_local {
                        let selected = state
                            .focused_local_branch
                            .as_deref()
                            .map(|name| name == branch.as_str())
                            .unwrap_or_else(|| {
                                selected_local_branch
                                    .map(|name| name == branch.as_str())
                                    .unwrap_or(false)
                            });
                        let response = ui.selectable_label(selected, branch);
                        if response.clicked() {
                            state.focused_local_branch = Some(branch.clone());
                            state.focused_remote_branch = None;
                        }
                        if response.double_clicked() {
                            action = Some(BranchSelectorAction::CheckoutLocalBranch {
                                name: branch.clone(),
                            });
                        }
                    }
                }
            });

            ui.collapsing("Remote", |ui| {
                if filtered_remote.is_empty() {
                    ui.label("No remote branches");
                } else {
                    for branch in &filtered_remote {
                        let selected = state
                            .focused_remote_branch
                            .as_deref()
                            .map(|name| name == branch.as_str())
                            .unwrap_or(false);
                        let response = ui.selectable_label(selected, branch);
                        if response.clicked() {
                            state.focused_remote_branch = Some(branch.clone());
                            state.focused_local_branch = None;
                        }
                        if response.double_clicked() {
                            action = Some(BranchSelectorAction::CheckoutRemoteBranch {
                                name: branch.clone(),
                            });
                        }
                    }
                }
            });
        });

    if state.show_new_branch_dialog {
        let mut open = true;
        let mut close = false;

        egui::Window::new("New Branch")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.label("Create from current branch");
                ui.add_sized(
                    [280.0, 28.0],
                    egui::TextEdit::singleline(&mut state.new_branch_name)
                        .hint_text("branch name"),
                );
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        let branch_name = state.new_branch_name.trim().to_string();
                        if !branch_name.is_empty() {
                            action = Some(BranchSelectorAction::CreateBranch {
                                name: branch_name,
                            });
                            state.new_branch_name.clear();
                            close = true;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        state.new_branch_name.clear();
                        close = true;
                    }
                });
            });

        if close || !open {
            state.show_new_branch_dialog = false;
        }
    }

    if state.show_checkout_dialog {
        let mut open = true;
        let mut close = false;

        egui::Window::new("Checkout")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.label("Enter reference (branch, tag) name or commit hash:");
                ui.add_sized(
                    [520.0, 34.0],
                    egui::TextEdit::singleline(&mut state.checkout_reference),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        state.checkout_reference.clear();
                        close = true;
                    }
                    if ui.button("OK").clicked() {
                        let reference = state.checkout_reference.trim().to_string();
                        if !reference.is_empty() {
                            action = Some(BranchSelectorAction::CheckoutReference { reference });
                            state.checkout_reference.clear();
                            close = true;
                        }
                    }
                });
            });

        if close || !open {
            state.show_checkout_dialog = false;
        }
    }

    action
}

fn draw_module_divider(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}
