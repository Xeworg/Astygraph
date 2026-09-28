//! Minimal native eframe/egui application shell.
//!
//! The UI layer delegates to [`AppState`][super::app_state::AppState],
//! keeping the state machine fully headless-testable.

use eframe::egui;
use rfd::FileDialog;

use crate::app_state::{AppState, FolderState};

/// Minimal desktop application.
#[derive(Default)]
pub struct AstynexApp {
    state: AppState,
}

impl eframe::App for AstynexApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading(crate::APPLICATION_NAME);

        match &self.state.folder_state {
            FolderState::Idle => {
                if ui.button("📁 Open Folder").clicked() {
                    self.state = self.state.open_folder();

                    // Immediately invoke the native picker and synchronise
                    // the result back into the state machine.
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
            FolderState::Loaded { path } => {
                ui.label(format!("📂 {}", path.display()));
                if ui.button("Close Folder").clicked() {
                    self.state = self.state.reset_folder();
                }
                ui.label("Project explorer coming soon.");
            }
        }
    }
}
