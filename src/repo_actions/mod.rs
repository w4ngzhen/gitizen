use eframe::egui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoAction {
    Fetch,
    PullMerge,
    PullRebase,
    Commit,
    Push,
}

pub fn render_inline(ui: &mut egui::Ui) -> Option<RepoAction> {
    let mut action = None;

    if ui.button("Fetch").clicked() {
        action = Some(RepoAction::Fetch);
    }
    if ui.button("Pull(M)").clicked() {
        action = Some(RepoAction::PullMerge);
    }
    if ui.button("Pull(R)").clicked() {
        action = Some(RepoAction::PullRebase);
    }
    if ui.button("Commit").clicked() {
        action = Some(RepoAction::Commit);
    }
    if ui.button("Push").clicked() {
        action = Some(RepoAction::Push);
    }

    action
}
