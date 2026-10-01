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
use crate::i18n::{I18n, Locale};
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
    expand_dir: Option<PathBuf>,
    collapse_dir: Option<PathBuf>,
    open_locale_dialog: bool,
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

        // Reserve a widget rectangle so the path is translated from SVG-local
        // coordinates into the current egui layout instead of being painted at
        // the window origin.
        let (rect, _response) =
            ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        let path_pts = path_pts
            .into_iter()
            .map(|point| egui::pos2(rect.left() + point.x, rect.top() + point.y))
            .collect();

        let stroke = ui.style().visuals.text_color();
        let shape = egui::Shape::Path(egui::epaint::PathShape::line(
            path_pts,
            egui::Stroke::new(icons::STROKE_WIDTH, stroke),
        ));
        ui.painter().add(shape);
    }

    /// Render a single tree node (directory or file) with expand/collapse affordance.
    ///
    /// For directories: shows expand/collapse icon and recursively renders children when expanded.
    /// For files: shows file icon and selection.
    /// For symlinks: shows link icon (non-expandable, non-selectable for navigation).
    fn render_tree_node(
        &mut self,
        ui: &mut egui::Ui,
        entries: &[crate::fs::Entry],
        selected_file: &Option<PathBuf>,
        _root: &PathBuf,
        actions: &mut UiActions,
        indent_level: usize,
    ) {
        // Indentation per level
        let indent_width = 20.0_f32;
        let total_indent = indent_level as f32 * indent_width;

        for entry in entries {
            let name = entry
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            let is_selected = selected_file.as_ref() == Some(&entry.path);
            let is_dir = entry.kind == FileKind::Dir;
            let is_symlink = entry.kind == FileKind::Symlink;

            // Determine if this directory is expanded
            let is_expanded = self.state.tree_cache().is_expanded(&entry.path);

            // Determine if this directory has cached children (was expanded before)
            let has_cached_children = self.state.tree_cache().is_dir_loaded(&entry.path);

            // Horizontal layout for this row
            ui.horizontal(|ui| {
                // Indentation
                if total_indent > 0.0 {
                    ui.add_space(total_indent);
                }

                if is_dir {
                    // Expand/collapse affordance
                    let arrow_char = if is_expanded { "▼" } else { "▶" };
                    let affordance_response =
                        ui.add(egui::Label::new(arrow_char).sense(egui::Sense::click()));

                    // Draw folder icon (open or closed based on expanded state)
                    let icon = if is_expanded {
                        Icon::FolderOpen
                    } else {
                        Icon::Folder
                    };
                    self.draw_icon(icon, ui, icons::ICON_SIZE);

                    // Directory name
                    let response = ui.selectable_label(is_selected, name.to_string());

                    // Handle interactions
                    if affordance_response.clicked() {
                        if is_expanded {
                            actions.collapse_dir = Some(entry.path.clone());
                        } else {
                            actions.expand_dir = Some(entry.path.clone());
                        }
                    }

                    if response.clicked() {
                        // Toggle expand/collapse on name click
                        if is_expanded {
                            actions.collapse_dir = Some(entry.path.clone());
                        } else {
                            actions.expand_dir = Some(entry.path.clone());
                        }
                    }
                } else {
                    // Non-directory entry: no expand affordance
                    ui.add_space(16.0); // Space for alignment with expandable items

                    let icon = if is_symlink {
                        Icon::Link
                    } else {
                        Icon::Document
                    };
                    self.draw_icon(icon, ui, icons::ICON_SIZE);

                    let response = ui.selectable_label(is_selected, name.to_string());

                    // File selection on click
                    if response.clicked() && !is_symlink && entry.kind == FileKind::File {
                        actions.selected_file = Some(entry.path.clone());
                    }
                    // Symlinks are displayed but not selectable
                }
            });

            // Recursively render children if directory is expanded
            if is_dir && is_expanded && has_cached_children {
                // Get children from tree cache (mutable access needed for get_entries)
                let children: Vec<crate::fs::Entry> = self
                    .state
                    .tree_cache_mut()
                    .get_entries(&entry.path)
                    .map(|e| e.to_vec())
                    .unwrap_or_default();

                // Render children with increased indent
                self.render_tree_node(
                    ui,
                    &children,
                    selected_file,
                    _root,
                    actions,
                    indent_level + 1,
                );
            }
        }
    }
}

impl eframe::App for AstynexApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Actions to apply after rendering (avoids borrow conflict with closures).
        let mut actions = UiActions::default();

        // --- Persistent Top Toolbar ---
        // Left: title + Open Folder button
        // Right: Language button
        ui.horizontal(|ui| {
            // Title on the left
            ui.heading(self.i18n.t("app.title").as_ref());

            // Open Folder button - always visible, works from any state
            ui.separator();
            if ui.button(self.i18n.t("folder.open").as_ref()).clicked() {
                self.state = self.state.open_folder();
                let picked = FileDialog::new()
                    .set_title(self.i18n.t("folder.open_title").as_ref())
                    .pick_folder();
                self.state = self.state.on_folder_selected(picked);
            }

            // Push remaining space
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                ui.add_space(10.0);

                // Language button on the right
                if ui.button(self.i18n.t("locale.toggle").as_ref()).clicked() {
                    actions.open_locale_dialog = true;
                }
            });
        });

        ui.separator();

        match &self.state.folder_state {
            FolderState::Idle => {
                ui.label(self.i18n.t("source.select_file").as_ref());
            }
            FolderState::Loading => {
                ui.spinner();
                ui.label(self.i18n.t("folder.opening").as_ref());
            }
            FolderState::Loaded {
                root,
                current_dir: _,
                entries,
                selected_file,
            } => {
                // Clone data for use inside closures.
                let entries = entries.clone();
                let selected_file = selected_file.clone();
                let root = root.clone();
                let tree_cache_incomplete = self.state.tree_cache().is_incomplete();

                // --- Left panel: explorer (expandable tree) ---
                egui::containers::panel::Panel::left("explorer")
                    .min_size(200.0)
                    .max_size(400.0)
                    .show(ui, |ui| {
                        ui.heading(self.i18n.t("explorer.title").as_ref());
                        ui.separator();

                        if ui.button(self.i18n.t("folder.close").as_ref()).clicked() {
                            actions.close_folder = true;
                        }

                        ui.separator();

                        // Incomplete discovery warning from tree cache.
                        if tree_cache_incomplete {
                            let msg = self.i18n.t("explorer.discovery_incomplete");
                            ui.label(egui::RichText::new(msg.as_ref()).color(Color32::YELLOW));
                        }

                        // Expandable tree rendering.
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            self.render_tree_node(
                                ui,
                                &entries,
                                &selected_file,
                                &root,
                                &mut actions,
                                0, // root level indent
                            );
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

        // --- Locale Dialog Modal ---
        // Render the modal BEFORE applying frame actions, so Apply/Cancel clicks
        // inside it are captured and applied in the same frame.
        //
        // Step 1: open the dialog if requested (takes effect before modal render).
        if actions.open_locale_dialog {
            self.state = self.state.open_locale_dialog();
        }

        // Local draft, independent of the frame-level `actions` struct.
        // Initialised from pending_locale so opening the dialog after a prior
        // partial selection preserves that selection.
        let mut draft_locale: Locale = self.state.pending_locale().unwrap_or(self.state.locale());

        let mut modal_apply = false;
        let mut modal_cancel = false;

        if self.state.is_locale_dialog_open() {
            let current_locale = self.state.locale();

            let response = egui::Modal::new(egui::Id::new("locale_dialog")).show(ui.ctx(), |ui| {
                ui.set_width(280.0);
                ui.heading(self.i18n.t("locale.dialog.title").as_ref());
                ui.separator();

                // Radio buttons bound directly to the local `draft_locale`.
                ui.radio_value(&mut draft_locale, Locale::En, {
                    let label = self.i18n.t("locale.dialog.english").as_ref().to_string();
                    if current_locale == Locale::En {
                        format!("{} {}", label, self.i18n.t("locale.dialog.current"))
                    } else {
                        label
                    }
                });
                ui.radio_value(&mut draft_locale, Locale::Es, {
                    let label = self.i18n.t("locale.dialog.spanish").as_ref().to_string();
                    if current_locale == Locale::Es {
                        format!("{} {}", label, self.i18n.t("locale.dialog.current"))
                    } else {
                        label
                    }
                });

                ui.add_space(10.0);
                ui.separator();

                // Apply / Cancel buttons — write into the outer frame actions.
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui
                            .button(self.i18n.t("locale.dialog.cancel").as_ref())
                            .clicked()
                        {
                            modal_cancel = true;
                        }
                        if ui
                            .button(self.i18n.t("locale.dialog.apply").as_ref())
                            .clicked()
                        {
                            modal_apply = true;
                        }
                    });
                });
            });

            // Backdrop click or Escape also cancels.
            if response.should_close() {
                modal_cancel = true;
            }
        }

        // --- Apply frame actions AFTER modal render ---
        //
        // Non-modal actions (folder/browser) were collected into `actions` by
        // their respective closures.  Apply them here before the modal actions
        // so that folder state is consistent when the i18n instance is rebuilt.
        if actions.close_folder {
            self.state = self.state.reset_folder();
        }
        if let Some(path) = actions.selected_file {
            self.state = self.state.select_file(Some(path));
        }
        if let Some(dir) = actions.expand_dir {
            self.state = self.state.expand_dir(&dir);
        }
        if let Some(dir) = actions.collapse_dir {
            self.state = self.state.collapse_dir(&dir);
        }

        // Modal actions — captured after render so they take effect this frame.
        //
        // Propagate the radio selection to draft_locale first, then handle
        // Apply / Cancel in order so that "select + Apply in one frame" works.
        if self.state.is_locale_dialog_open() {
            self.state = self.state.draft_locale(draft_locale);
        }
        if modal_apply {
            let new_locale = self.state.pending_locale().unwrap_or(self.state.locale());
            self.state = self.state.apply_locale();
            self.i18n = I18n::new(new_locale);
        }
        if modal_cancel {
            self.state = self.state.cancel_locale();
        }
    }
}
