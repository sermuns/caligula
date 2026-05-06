use egui::{
    CentralPanel, Checkbox, Color32, ComboBox, MenuBar, Panel, RichText, UiBuilder, ViewportCommand,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::{
    compression::CompressionFormat,
    device::{self, Removable, WriteTarget, enumerate_devices},
    hash::{HashAlg, parse_hash_input},
    logging::LogPaths,
    orchestrator::{Orchestrator, WriteVerifyParams, WriterState},
    runtime::RemoteSpawn,
};

pub struct App<O: Orchestrator, R: RemoteSpawn> {
    pub log_paths: Arc<LogPaths>,
    pub options: Options,
    pub ongoing_write: Arc<Mutex<Option<OngoingWrite>>>,
    pub orc: Arc<O>,
    pub runtime: R,
}

pub struct OngoingWrite {
    pub write_progress: u64,
    pub verify_progress: u64,
}

#[derive(Default)]
#[cfg_attr(debug_assertions, derive(serde::Deserialize, serde::Serialize))]
pub struct Options {
    pub picked_image: Option<PathBuf>,
    pub file_hash: FileHashOptions,
    pub detected_compression_format: Option<CompressionFormat>,
    #[cfg_attr(debug_assertions, serde(skip))]
    pub possible_write_targets: Vec<WriteTarget>,
    #[cfg_attr(debug_assertions, serde(skip))]
    pub selected_write_target: Option<WriteTarget>,
    pub show_all_disks: bool,
    #[cfg_attr(debug_assertions, serde(skip))]
    pub has_confirmed_writing: bool,
    #[cfg_attr(debug_assertions, serde(skip))]
    pub write_verify_params: Option<WriteVerifyParams>,
}

#[derive(Default)]
#[cfg_attr(debug_assertions, derive(serde::Deserialize, serde::Serialize))]
pub struct FileHashOptions {
    pub entered_hash: String,
    pub possible_algorithms: Vec<HashAlg>,
    pub selected_algorithm: Option<HashAlg>,
    pub last_error: String,
    pub skip: bool,
    #[cfg_attr(debug_assertions, serde(skip))]
    pub verified: bool,
}

impl<O: Orchestrator, R: RemoteSpawn> App<O, R> {
    pub fn new(
        cc: &eframe::CreationContext,
        runtime: R,
        orc: Arc<O>,
        log_paths: Arc<LogPaths>,
    ) -> Self {
        #[cfg(not(debug_assertions))]
        let options = Options::default();

        #[cfg(debug_assertions)]
        let options: Options = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        let mut s = Self {
            log_paths,
            options,
            ongoing_write: Arc::new(Mutex::new(None)),
            orc,
            runtime,
        };

        s.refresh_devices();

        s
    }

    pub fn refresh_devices(&mut self) {
        // TODO: deduplicate this.
        // This is code stolen from `ask_outfile.rs`!
        self.options.possible_write_targets = enumerate_devices()
            .filter(|d| self.options.show_all_disks || d.removable == Removable::Yes)
            .collect();
        self.options.possible_write_targets.sort();
    }

    pub fn file_hash_is_verified_or_skipped(&self) -> bool {
        self.options.file_hash.verified || self.options.file_hash.skip
    }

    pub fn is_ready_for_writing(&self) -> bool {
        self.options.picked_image.is_some()
            && self.file_hash_is_verified_or_skipped()
            && self.options.selected_write_target.is_some()
    }

    pub fn add_file_hash_ui(&mut self, ui: &mut egui::Ui) {
        let FileHashOptions {
            entered_hash: file_hash_str,
            possible_algorithms: file_hash_algorithms_possible,
            selected_algorithm: file_hash_algorithm_selected,
            last_error: latest_hashing_error,
            skip: skip_hashing,
            verified: verified_hash,
        } = &mut self.options.file_hash;

        ui.horizontal(|ui| {
            ui.strong("File hash");
            ui.checkbox(skip_hashing, "Skip?");
        });

        ui.scope_builder(
            UiBuilder {
                disabled: *skip_hashing,
                // invisible: *skip_hashing,
                ..Default::default()
            },
            |ui| {
                ui.label("We will guess the hash algorithm from your input.");
                if ui.text_edit_singleline(file_hash_str).changed() {
                    match parse_hash_input(file_hash_str) {
                        Ok((algs, _)) => {
                            if algs.len() == 1 {
                                *file_hash_algorithm_selected = Some(algs[0]);
                            }
                            *file_hash_algorithms_possible = algs;
                            latest_hashing_error.clear();
                        }
                        Err(e) => {
                            *file_hash_algorithms_possible = vec![];
                            *file_hash_algorithm_selected = None;
                            *latest_hashing_error = e.to_string();
                        }
                    }
                }

                if latest_hashing_error.is_empty() {
                    ui.horizontal(|ui| {
                        for alg in file_hash_algorithms_possible {
                            let is_selected = Some(*alg) == *file_hash_algorithm_selected;

                            if ui.selectable_label(is_selected, alg.to_string()).clicked() {
                                *file_hash_algorithm_selected = Some(*alg);
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.add_enabled(false, Checkbox::without_text(verified_hash));
                        if ui.button("Verify").clicked() {
                            // TODO:
                            *verified_hash = true;
                        }
                    });
                } else if *skip_hashing {
                    ui.label("");
                } else {
                    ui.label(RichText::new(&*latest_hashing_error).color(Color32::RED));
                }
            },
        );
    }

    pub fn add_image_ui(&mut self, ui: &mut egui::Ui) {
        let Options {
            detected_compression_format,
            picked_image,
            ..
        } = &mut self.options;

        ui.strong("Image");
        if ui.button("💿 Pick file").clicked()
            && let Some(picked) = rfd::FileDialog::new().pick_file()
        {
            *detected_compression_format = CompressionFormat::detect_from_path(&picked);
            *picked_image = Some(picked);
        }
        if let Some(picked) = picked_image {
            ui.label(picked.to_string_lossy());
            if let Some(cf) = detected_compression_format {
                ui.label(format!("Detected compression format: {}", cf));
            } else {
                ui.label(
                RichText::new(
                    "Couldn't detect compression format for picked image, assuming uncompressed!",
                )
                .color(Color32::YELLOW),
            );
            }
        }
    }

    pub fn add_target_disk_ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Target disk");
        if ui.button("Refresh devices").clicked() {
            self.refresh_devices();
        }

        // FIXME:
        // - stop alloc:ing and doing so much work here.. DON'T CLONE!
        // - move the label formatting into a place where it's done ONCE, not on every ui render!
        // - deduplicate, label formatting is stolen from `ask_outfile.rs`
        ComboBox::from_label(format!(
            "{} available",
            self.options.possible_write_targets.len()
        ))
        .selected_text(
            self.options
                .selected_write_target
                .as_ref()
                .map(|dev| match dev.target_type {
                    device::Type::Disk => format!(
                        "{} | {} - {} ({}, removable: {})",
                        dev.name, dev.model, dev.size, dev.target_type, dev.removable
                    ),
                    _ => format!(
                        "{} | {} - {} ({})",
                        dev.name, dev.model, dev.size, dev.target_type
                    ),
                })
                .unwrap_or_default(),
        )
        .show_ui(ui, |ui| {
            for dev in &self.options.possible_write_targets {
                let label = match dev.target_type {
                    device::Type::Disk => format!(
                        "{} | {} - {} ({}, removable: {})",
                        dev.name, dev.model, dev.size, dev.target_type, dev.removable
                    ),
                    _ => format!(
                        "{} | {} - {} ({})",
                        dev.name, dev.model, dev.size, dev.target_type
                    ),
                };
                ui.selectable_value(
                    &mut self.options.selected_write_target,
                    Some(dev.clone()),
                    label,
                );
            }
        });
    }

    pub fn add_begin_writing_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_enabled_ui(self.is_ready_for_writing(), |ui| {
            ui.strong("Write");
            if ui.button("Prepare for writing").clicked() {
                // FIXME:
                // don't unwrap.
                // actually don't even have this shitty refresh button,
                // should just refresh when any of the underlying values change
                self.options.write_verify_params = WriteVerifyParams::new(
                    self.options.picked_image.clone().unwrap(),
                    self.options.detected_compression_format.unwrap(),
                    self.options.selected_write_target.clone().unwrap(),
                )
                .ok();
            }

            if let Some(write_verify_params) = &self.options.write_verify_params {
                // TODO: show summary!
                // ui.label(write_verify_params.to_string());

                ui.label(RichText::new("Ready to write!").color(Color32::GREEN));

                if !self.options.has_confirmed_writing {
                    if ui.button("Perform write").clicked() {
                        self.options.has_confirmed_writing = true;
                    }
                    return;
                }

                ui.label(
                    RichText::new("THIS ACTION WILL DESTROY ALL DATA ON THIS DEVICE!!!")
                        .color(Color32::YELLOW),
                );

                if ui.button("I know, do it!").clicked() {
                    // TODO: make sure this really needs to clone
                    let log_paths = self.log_paths.clone();
                    let write_verify_params = self.options.write_verify_params.take();
                    let cf = self.options.detected_compression_format.unwrap(); // FIXME:
                    let ongoing_write = self.ongoing_write.clone();
                    let egui_ctx = ui.ctx().clone();

                    *ongoing_write.lock().unwrap() = Some(OngoingWrite {
                        write_progress: 0,
                        verify_progress: 0,
                    });

                    loop {
                        let x = handle.events.next().await;
                        info!(?x, "got event from burn handle");
                        child_state = child_state.on_status(Instant::now(), x);
                        // FIXME: fugly-ass unwrselfing
                        match &child_state {
                            WriterState::Writing(b) => {
                                ongoing_write
                                    .lock()
                                    .unwrap()
                                    .as_mut()
                                    .unwrap()
                                    .write_progress = (b.approximate_ratio() * 1000.0) as u64
                            }
                            WriterState::Verifying {
                                total_write_bytes, ..
                            } => {
                                ongoing_write
                                    .lock()
                                    .unwrap()
                                    .as_mut()
                                    .unwrap()
                                    .verify_progress = total_write_bytes * 1000 / input_file_bytes
                            }
                            WriterState::Finished { .. } => break,
                        }
                        egui_ctx.request_repaint();
                    }
                }
            }
        });
    }
}

impl<O: Orchestrator, R: RemoteSpawn> eframe::App for App<O, R> {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {}

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        const SECTION_SPACING: f32 = 6.;

        Panel::top("top_menu").show_inside(ui, |ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("❌ Quit").clicked() {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                    }
                });
            });
        });

        CentralPanel::default().show_inside(ui, |ui| {
            if let Some(ongoing_write) = &*self.ongoing_write.lock().unwrap() {
                ui.label("writing!!");
                ui.label(format!("Write progress: {}%", ongoing_write.write_progress));
                ui.label(format!(
                    "Verify progress: {}%",
                    ongoing_write.verify_progress
                ));

                return;
            }

            ui.label(RichText::new(env!("CARGO_PKG_NAME")).heading().size(26.));
            ui.label(env!("CARGO_PKG_DESCRIPTION"));

            ui.add_space(SECTION_SPACING * ui.spacing().item_spacing.y);

            self.add_image_ui(ui);

            ui.add_space(SECTION_SPACING * ui.spacing().item_spacing.y);

            ui.add_enabled_ui(self.options.picked_image.is_some(), |ui| {
                self.add_file_hash_ui(ui)
            });

            ui.add_space(SECTION_SPACING * ui.spacing().item_spacing.y);

            ui.add_enabled_ui(self.file_hash_is_verified_or_skipped(), |ui| {
                self.add_target_disk_ui(ui)
            });

            ui.add_space(SECTION_SPACING * ui.spacing().item_spacing.y);

            ui.add_enabled_ui(self.options.selected_write_target.is_some(), |ui| {
                self.add_begin_writing_ui(ui)
            });
        });
    }

    #[cfg(not(debug_assertions))]
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    #[cfg(debug_assertions)]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.options);
    }
}
