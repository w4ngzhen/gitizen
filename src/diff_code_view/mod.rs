use eframe::egui;
use egui::Color32;
use egui_extras::syntax_highlighting::{self, CodeTheme};

use crate::git_status::{SplitCellKind, SplitDiffModel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRenderMode {
    Unified,
    Split,
}

impl Default for DiffRenderMode {
    fn default() -> Self {
        Self::Unified
    }
}

pub fn render_mode_switch(ui: &mut egui::Ui, mode: &mut DiffRenderMode) {
    ui.horizontal(|ui| {
        ui.label("View:");
        ui.selectable_value(mode, DiffRenderMode::Unified, "Unified");
        ui.selectable_value(mode, DiffRenderMode::Split, "Split");
    });
}

pub fn render(
    ui: &mut egui::Ui,
    diff_text: &str,
    split_model: Option<&SplitDiffModel>,
    mode: DiffRenderMode,
) {
    match mode {
        DiffRenderMode::Unified => render_unified(ui, diff_text),
        DiffRenderMode::Split => render_split(ui, split_model),
    }
}

fn render_unified(ui: &mut egui::Ui, diff_text: &str) {
    let theme = CodeTheme::from_style(ui.style());
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut job =
                syntax_highlighting::highlight(ui.ctx(), ui.style(), &theme, diff_text, "diff");
            job.wrap.max_width = f32::INFINITY;
            ui.add(egui::Label::new(job).selectable(true).extend());
        });
}

fn render_split(ui: &mut egui::Ui, split_model: Option<&SplitDiffModel>) {
    let Some(model) = split_model else {
        ui.label("No split diff data");
        return;
    };

    let theme = CodeTheme::from_style(ui.style());
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("split_diff_grid")
                .striped(false)
                .num_columns(2)
                .min_col_width(ui.available_width() * 0.48)
                .show(ui, |ui| {
                    for (idx, row) in model.rows.iter().enumerate() {
                        render_cell(
                            ui,
                            &theme,
                            &row.left,
                            &row.language,
                            cell_bg(row.left_kind),
                            idx,
                            "L",
                        );
                        render_cell(
                            ui,
                            &theme,
                            &row.right,
                            &row.language,
                            cell_bg(row.right_kind),
                            idx,
                            "R",
                        );
                        ui.end_row();
                    }
                });
        });
}

fn render_cell(
    ui: &mut egui::Ui,
    theme: &CodeTheme,
    text: &str,
    language: &str,
    bg: Option<Color32>,
    row_idx: usize,
    side: &str,
) {
    let source = if text.is_empty() { " " } else { text };
    let mut job = syntax_highlighting::highlight(ui.ctx(), ui.style(), theme, source, language);
    job.wrap.max_width = f32::INFINITY;

    let frame = egui::Frame::NONE
        .fill(bg.unwrap_or(Color32::TRANSPARENT))
        .inner_margin(egui::Margin::symmetric(4, 1));

    frame.show(ui, |ui| {
        ui.push_id(("split-row", row_idx, side), |ui| {
            ui.add(egui::Label::new(job).selectable(true).extend());
        });
    });
}

fn cell_bg(kind: SplitCellKind) -> Option<Color32> {
    match kind {
        SplitCellKind::Added => Some(Color32::from_rgb(28, 58, 39)),
        SplitCellKind::Removed => Some(Color32::from_rgb(61, 31, 31)),
        SplitCellKind::Meta => Some(Color32::from_rgb(36, 39, 46)),
        SplitCellKind::None | SplitCellKind::Context => None,
    }
}
