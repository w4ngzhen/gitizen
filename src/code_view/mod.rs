use std::path::Path;

use eframe::egui;
use egui_extras::syntax_highlighting::{self, CodeTheme};

pub fn render_readonly_code(ui: &mut egui::Ui, text: &str, path: Option<&str>) {
    let theme = CodeTheme::from_style(ui.style());
    let language = language_from_path(path);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut job = syntax_highlighting::highlight(ui.ctx(), ui.style(), &theme, text, language);
            job.wrap.max_width = f32::INFINITY;
            ui.add(egui::Label::new(job).selectable(true).extend());
        });
}

fn language_from_path(path: Option<&str>) -> &'static str {
    let Some(path) = path else {
        return "";
    };

    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match extension.as_str() {
        "rs" => "rust",
        "toml" => "toml",
        "py" => "python",
        "c" | "h" | "hpp" | "cpp" | "cc" | "cxx" => "cpp",
        "js" => "javascript",
        "ts" => "typescript",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "md" | "markdown" => "markdown",
        "sh" | "bash" | "zsh" => "bash",
        _ => "",
    }
}
