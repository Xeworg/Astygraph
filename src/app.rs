//! Minimal native eframe/egui application shell.
//!
//! The UI layer delegates to [`AppState`][super::app_state::AppState] and
//! the headless [`fs`][super::fs] and [`viewer`][super::viewer] modules,
//! keeping filesystem/viewer core fully headless-testable.
//!
//! All user-visible strings are retrieved from [`crate::i18n`] — no hardcoded
//! widget text.  Icon glyphs use Heroicons outline SVGs from [`crate::icons`].

use std::path::PathBuf;

use eframe::egui;
use eframe::epaint::Color32;
use rfd::FileDialog;

use crate::app_state::{AppState, FolderState};
use crate::fs::FileKind;
use crate::i18n::I18n;
use crate::icons::{self, Icon};
use crate::viewer::{self, TextContent};

/// Minimal desktop application — split pane: left browser, right source viewer.
pub struct AstynexApp {
    state: AppState,
    i18n: I18n,
}

impl Default for AstynexApp {
    fn default() -> Self {
        let state = AppState::default();
        Self {
            i18n: I18n::new(state.locale()),
            state,
        }
    }
}

/// Actions produced by one UI frame, applied after closures complete.
#[derive(Default)]
struct UiActions {
    close_folder: bool,
    selected_file: Option<PathBuf>,
    navigate_to_dir: Option<PathBuf>,
    navigate_up: bool,
    toggle_locale: bool,
}

impl AstynexApp {
    /// Draw a Heroicons outline icon at the current cursor position.
    ///
    /// Advances the cursor by `size` points to the right so the next widget
    /// appears after the icon.
    fn draw_icon(&self, icon: Icon, ui: &mut egui::Ui, size: f32) {
        let path_pts = icons::parse_path(icon.path_data(), size);

        if path_pts.len() < 2 {
            return;
        }

        // Draw as a stroked path in the current text color.
        let stroke = ui.style().visuals.text_color();
        let shape = egui::Shape::Path(egui::epaint::PathShape::line(
            path_pts,
            egui::Stroke::new(icons::STROKE_WIDTH, stroke),
        ));
        ui.painter().add(shape);

        // Advance cursor horizontally.
        ui.set_min_size([size + 4.0, ui.min_size().y].into());
    }
}

impl eframe::App for AstynexApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading(self.i18n.t("app.title").as_ref());

        // Actions to apply after rendering (avoids borrow conflict with closures).
        let mut actions = UiActions::default();

        // --- Locale toggle ---
        // Small button in the top-right area.
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                if ui
                    .button(format!("[{}]", self.state.locale().label()))
                    .clicked()
                {
                    actions.toggle_locale = true;
                }
            });
        });

        match &self.state.folder_state {
            FolderState::Idle => {
                if ui.button(self.i18n.t("folder.open").as_ref()).clicked() {
                    self.state = self.state.open_folder();
                    let picked = FileDialog::new()
                        .set_title(self.i18n.t("folder.open_title").as_ref())
                        .pick_folder();
                    self.state = self.state.on_folder_selected(picked);
                }
            }
            FolderState::Loading => {
                ui.spinner();
                ui.label(self.i18n.t("folder.opening").as_ref());
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
                        ui.heading(self.i18n.t("explorer.title").as_ref());
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
                        if !at_root {
                            ui.horizontal(|ui| {
                                if ui.button(self.i18n.t("explorer.up").as_ref()).clicked() {
                                    actions.navigate_up = true;
                                }
                            });
                        }

                        if ui.button(self.i18n.t("folder.close").as_ref()).clicked() {
                            actions.close_folder = true;
                        }

                        ui.separator();

                        // Incomplete discovery warning.
                        let is_incomplete = entries.iter().any(|e| e.is_incomplete);
                        if is_incomplete {
                            let msg = self.i18n.t("explorer.discovery_incomplete");
                            ui.label(egui::RichText::new(msg.as_ref()).color(Color32::YELLOW));
                        }

                        // File/directory listing.
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            // ".." entry to go up one level (omitted at root).
                            if !at_root {
                                let is_selected = false;
                                if ui
                                    .selectable_label(
                                        is_selected,
                                        self.i18n.t("explorer.parent_dir").as_ref(),
                                    )
                                    .clicked()
                                {
                                    actions.navigate_up = true;
                                }
                            }

                            for entry in &entries {
                                let name = entry
                                    .path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");

                                let is_selected = selected_file.as_ref() == Some(&entry.path);

                                // Icon + name row.
                                ui.horizontal(|ui| {
                                    // Draw SVG icon matching file kind.
                                    let icon = match entry.kind {
                                        FileKind::Dir => Icon::Folder,
                                        FileKind::Symlink => Icon::Link,
                                        FileKind::File => Icon::Document,
                                    };
                                    self.draw_icon(icon, ui, icons::ICON_SIZE);

                                    let response =
                                        ui.selectable_label(is_selected, name.to_string());

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
                                });
                            }
                        });
                    });

                // --- Right panel: source viewer ---
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.heading(self.i18n.t("source.title").as_ref());

                    if let Some(selected) = &selected_file {
                        let file_name = selected.file_name().and_then(|n| n.to_str()).unwrap_or("");

                        ui.horizontal(|ui| {
                            self.draw_icon(Icon::Document, ui, icons::ICON_SIZE);
                            ui.label(file_name.to_string());

                            // Large-file diagnostic.
                            if let Ok(metadata) = std::fs::metadata(selected) {
                                if metadata.len() > viewer::LARGE_FILE_THRESHOLD {
                                    let diag = self.i18n.t("source.large_file");
                                    ui.label(
                                        egui::RichText::new(diag.as_ref()).color(Color32::YELLOW),
                                    );
                                }
                            }
                        });

                        ui.separator();

                        // Open and display the file.
                        match viewer::open_file(selected) {
                            Ok(TextContent::Empty) => {
                                ui.label(self.i18n.t("source.empty_file").as_ref());
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
                                let label = egui::RichText::new(
                                    self.i18n.t_args("source.error", &[&err]).as_ref(),
                                )
                                .color(Color32::RED);
                                ui.label(label);
                            }
                        }
                    } else {
                        ui.label(self.i18n.t("source.select_file").as_ref());
                    }
                });
            }
        }

        // Apply actions AFTER all closures (no borrow conflict).
        if actions.toggle_locale {
            let new_locale = self.state.locale().toggle();
            self.state = self.state.clone().with_locale(new_locale);
            self.i18n = I18n::new(new_locale);
        }
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
