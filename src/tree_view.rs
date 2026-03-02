use std::collections::BTreeSet;

use eframe::egui;

pub struct TreeItem<'a, T> {
    pub path: &'a str,
    pub payload: &'a T,
}

struct PreparedItem<'a, T> {
    raw_path: &'a str,
    normalized_path: String,
    payload: &'a T,
}

pub fn render_tree<T, A, FRenderFile, FDirContext, FFileContext>(
    ui: &mut egui::Ui,
    items: &[TreeItem<'_, T>],
    render_file: &mut FRenderFile,
    dir_context_menu: &mut FDirContext,
    file_context_menu: &mut FFileContext,
) -> Option<A>
where
    FRenderFile: FnMut(&mut egui::Ui, &str, &str, &T) -> (egui::Response, Option<A>),
    FDirContext: FnMut(&mut egui::Ui, &str, &str) -> Option<A>,
    FFileContext: FnMut(&mut egui::Ui, &str, &str, &T) -> Option<A>,
{
    let prepared: Vec<PreparedItem<'_, T>> = items
        .iter()
        .map(|item| PreparedItem {
            raw_path: item.path,
            normalized_path: normalize_path(item.path),
            payload: item.payload,
        })
        .collect();

    let mut action = None;
    render_node(
        ui,
        &prepared,
        "",
        &mut action,
        render_file,
        dir_context_menu,
        file_context_menu,
    );
    action
}

fn render_node<T, A, FRenderFile, FDirContext, FFileContext>(
    ui: &mut egui::Ui,
    items: &[PreparedItem<'_, T>],
    prefix: &str,
    action: &mut Option<A>,
    render_file: &mut FRenderFile,
    dir_context_menu: &mut FDirContext,
    file_context_menu: &mut FFileContext,
) where
    FRenderFile: FnMut(&mut egui::Ui, &str, &str, &T) -> (egui::Response, Option<A>),
    FDirContext: FnMut(&mut egui::Ui, &str, &str) -> Option<A>,
    FFileContext: FnMut(&mut egui::Ui, &str, &str, &T) -> Option<A>,
{
    let (dirs, leaf_files) = collect_tree_entries(items, prefix);

    for dir_name in dirs {
        let next_prefix = format!("{prefix}{dir_name}/");
        let response = egui::CollapsingHeader::new(&dir_name)
            .default_open(true)
            .show(ui, |ui| {
                render_node(
                    ui,
                    items,
                    &next_prefix,
                    action,
                    render_file,
                    dir_context_menu,
                    file_context_menu,
                );
            });

        response.header_response.context_menu(|ui| {
            if let Some(next_action) = dir_context_menu(ui, next_prefix.trim_end_matches('/'), &dir_name)
            {
                *action = Some(next_action);
            }
        });
    }

    for item in leaf_files {
        let file_name = item
            .normalized_path
            .rsplit('/')
            .next()
            .unwrap_or(&item.normalized_path);

        let (response, next_action) = render_file(ui, item.raw_path, file_name, item.payload);
        if next_action.is_some() {
            *action = next_action;
        }

        response.context_menu(|ui| {
            if let Some(next_action) = file_context_menu(ui, item.raw_path, file_name, item.payload) {
                *action = Some(next_action);
            }
        });
    }
}

fn collect_tree_entries<'a, T>(
    items: &'a [PreparedItem<'_, T>],
    prefix: &str,
) -> (Vec<String>, Vec<&'a PreparedItem<'a, T>>) {
    let mut dirs = BTreeSet::new();
    let mut leaf_files = Vec::new();

    for item in items {
        if !item.normalized_path.starts_with(prefix) {
            continue;
        }

        let rest = &item.normalized_path[prefix.len()..];
        if rest.is_empty() {
            continue;
        }

        match rest.split_once('/') {
            Some((dir, _)) => {
                dirs.insert(dir.to_string());
            }
            None => leaf_files.push(item),
        }
    }

    leaf_files.sort_by(|a, b| a.normalized_path.cmp(&b.normalized_path));
    (dirs.into_iter().collect(), leaf_files)
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}
