//! Minimal native eframe/egui application shell.
//!
//! The UI layer delegates to [`AppState`][super::app_state::AppState] and
//! the headless [`fs`][super::fs] and [`viewer`][super::viewer] modules,
//! keeping filesystem/viewer core fully headless-testable.

use std::path::PathBuf;

use eframe::egui;
use eframe::epaint::Color32;
use rfd::FileDialog;

use crate::app_state::{AppState, FolderState};
use crate::fs::FileKind;
use crate::viewer::{self, TextContent};

/// Minimal desktop application — split pane: left browser, right source viewer.
#[derive(Default)]
pub struct AstynexApp {
    state: AppState,
}

/// Actions produced by one UI frame, applied after closures complete.
#[derive(Default)]
struct UiActions {
    close_folder: bool,
    selected_file: Option<PathBuf>,
    navigate_to_dir: Option<PathBuf>,
    navigate_up: bool,
}

impl eframe::App for AstynexApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading(crate::APPLICATION_NAME);

        // Actions to apply after rendering (avoids borrow conflict with closures).
        let mut actions = UiActions::default();

        match &self.state.folder_state {
            FolderState::Idle => {
                if ui.button("Open Folder").clicked() {
                    self.state = self.state.open_folder();
                    let picked = FileDialog::new()
                        .set_title("Open Project Folder")
                        .pick_folder();
                    self.state = self.state.on_folder_selected(picked);
                }
            }
            FolderState::Loading => {
                ui.spinner();
                ui.label("Opening folder…");
            }
            FolderState::Loaded {
                root,
                current_dir,
                entries,
                selected_file,
            } => {
                // Clone data for use inside closures (no direct capture of self).
                let entries = entries.clone();
                let selected_file = selected_file.clone();
                let current_dir = current_dir.clone();
                let root = root.clone();

                // Determine if we are at the root (no ".." entry shown).
                let at_root = current_dir == root;

                // --- Left panel: explorer ---
                egui::containers::panel::Panel::left("explorer")
                    .min_size(200.0)
                    .max_size(400.0)
                    .show(ui, |ui| {
                        ui.heading("Explorer");
                        ui.separator();

                        // Breadcrumb: current directory path (truncated to last 2 segments).
                        let breadcrumb = if at_root {
                            current_dir
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| current_dir.to_string_lossy().to_string())
                        } else {
                            // Show root / current for deep paths.
                            let root_name = root
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| root.to_string_lossy().to_string());
                            let cur_name = current_dir
                                .file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| current_dir.to_string_lossy().to_string());
                            format!("{root_name} / {cur_name}")
                        };
                        ui.label(egui::RichText::new(&breadcrumb).small());

                        // Navigate-up button (shown when not at root).
                        if !at_root && ui.button("Up").clicked() {
                            actions.navigate_up = true;
                        }

                        if ui.button("Close Folder").clicked() {
                            actions.close_folder = true;
                        }

                        ui.separator();

                        // Incomplete discovery warning.
                        let is_incomplete = entries.iter().any(|e| e.is_incomplete);
                        if is_incomplete {
                            let msg = "[!] Discovery incomplete — 50,000-entry limit reached.";
                            ui.label(egui::RichText::new(msg).color(Color32::YELLOW));
                        }

                        // File/directory listing.
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            // ".." entry to go up one level (omitted at root).
                            if !at_root {
                                let is_selected = false;
                                if ui.selectable_label(is_selected, "[..]").clicked() {
                                    actions.navigate_up = true;
                                }
                            }

                            for entry in &entries {
                                let name = entry
                                    .path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");

                                let prefix = match entry.kind {
                                    FileKind::Dir => "[DIR]  ",
                                    FileKind::Symlink => "[LINK]",
                                    FileKind::File => "       ",
                                };

                                let is_selected = selected_file.as_ref() == Some(&entry.path);

                                let response =
                                    ui.selectable_label(is_selected, format!("{prefix}{name}"));

                                if response.clicked() {
                                    if entry.kind == FileKind::Dir {
                                        // Clicking a directory navigates into it;
                                        // clears the viewer so the panel shows the new listing.
                                        actions.navigate_to_dir = Some(entry.path.clone());
                                        actions.selected_file = None;
                                    } else if entry.kind == FileKind::File {
                                        // Clicking a file selects it for the viewer.
                                        actions.selected_file = Some(entry.path.clone());
                                    }
                                    // Symlinks are displayed but not traversable on click.
                                }
                            }
                        });
                    });

                // --- Right panel: source viewer ---
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.heading("Source");

                    if let Some(selected) = &selected_file {
                        let file_name = selected.file_name().and_then(|n| n.to_str()).unwrap_or("");

                        ui.horizontal(|ui| {
                            ui.label(file_name.to_string());

                            // Large-file diagnostic.
                            if let Ok(metadata) = std::fs::metadata(selected) {
                                if metadata.len() > viewer::LARGE_FILE_THRESHOLD {
                                    let diag = viewer::large_file_diagnostic();
                                    ui.label(egui::RichText::new(diag).color(Color32::YELLOW));
                                }
                            }
                        });

                        ui.separator();

                        // Open and display the file.
                        match viewer::open_file(selected) {
                            Ok(TextContent::Empty) => {
                                ui.label("(empty file)");
                            }
                            Ok(TextContent::Lines(lines)) => {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    let line_width = 50.0;
                                    for line in &lines {
                                        ui.horizontal(|ui| {
                                            // Line number.
                                            ui.add_sized(
                                                [line_width, ui.available_height()],
                                                egui::Label::new(
                                                    egui::RichText::new(format!(
                                                        "{:>5} ",
                                                        line.number,
                                                    ))
                                                    .color(Color32::DARK_GRAY),
                                                ),
                                            );
                                            // Line text (monospace).
                                            ui.label(egui::RichText::new(&line.text).monospace());
                                        });
                                    }
                                });
                            }
                            Err(err) => {
                                let label = egui::RichText::new(format!("Error: {err}"))
                                    .color(Color32::RED);
                                ui.label(label);
                            }
                        }
                    } else {
                        ui.label("Select a file to view its contents.");
                    }
                });
            }
        }

        // Apply actions AFTER all closures (no borrow conflict).
        if actions.close_folder {
            self.state = self.state.reset_folder();
        }
        if actions.navigate_up {
            self.state = self.state.navigate_up();
        }
        if let Some(dir) = actions.navigate_to_dir {
            self.state = self.state.navigate_to_dir(&dir);
        }
        if let Some(path) = actions.selected_file {
            self.state = self.state.select_file(Some(path));
        }
    }
}
