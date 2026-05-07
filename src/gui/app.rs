use egui::{
    CentralPanel, Checkbox, Color32, ComboBox, MenuBar, Panel, ProgressBar, RichText, UiBuilder,
    ViewportCommand,
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};
use tracing::{error, info};

use crate::{
    compression::CompressionFormat,
    device::{self, Removable, WriteTarget, enumerate_devices},
    facade::{CaligulaFacade, WVState, WriteVerifyWorkflow, watch::Watch},
    hash::{HashAlg, parse_hash_input},
    logging::LogPaths,
    runtime::RemoteSpawn,
    ui::FacadeExt,
};

pub struct App {
    pub options: Options,
    pub main_to_worker_tx: Sender<WorkerEvent>,
    pub write_verify_state: Arc<Mutex<Option<Watch<WVState>>>>,
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

enum WorkerEvent {
    StartWrite(WriteVerifyWorkflow),
    Abort,
}

impl App {
    fn spawn_worker_thread(
        orc: Arc<impl CaligulaFacade>,
        runtime: impl RemoteSpawn + Send + 'static,
        ui_ctx: egui::Context,
        main_to_worker_rx: Receiver<WorkerEvent>,
        write_verify_state: Arc<Mutex<Option<Watch<WVState>>>>,
    ) {
        const REFRESH_PERIOD: Duration = Duration::from_millis(250);

        std::thread::spawn(move || {
            'outer: loop {
                let Ok(WorkerEvent::StartWrite(write_verify_workflow)) =
                    main_to_worker_rx.try_recv()
                else {
                    info!("nothing happening in worker thread, sleeping...");
                    std::thread::sleep(REFRESH_PERIOD);
                    continue;
                };

                let state = match orc
                    .clone()
                    .start_write_verify_blocking(&runtime, write_verify_workflow)
                {
                    Err(e) => {
                        error!(?e, "failed to start write/verify process");
                        continue;
                    }
                    Ok(state) => state,
                };

                write_verify_state.lock().unwrap().replace(state.clone());

                while !matches!(&*state.borrow(), WVState::Finished { .. }) {
                    ui_ctx.request_repaint();
                    std::thread::sleep(REFRESH_PERIOD);

                    if matches!(main_to_worker_rx.try_recv(), Ok(WorkerEvent::Abort)) {
                        write_verify_state.lock().unwrap().take();
                        ui_ctx.request_repaint();
                        continue 'outer;
                    }
                }

                while !matches!(main_to_worker_rx.try_recv(), Ok(WorkerEvent::Abort)) {
                    std::thread::sleep(REFRESH_PERIOD);
                }

                write_verify_state.lock().unwrap().take();
                ui_ctx.request_repaint();
            }
        });
    }

    pub fn new(
        cc: &eframe::CreationContext,
        runtime: impl RemoteSpawn + Send + 'static,
        orc: Arc<impl CaligulaFacade>,
        log_paths: Arc<LogPaths>,
    ) -> Self {
        #[cfg(not(debug_assertions))]
        let options = Options::default();

        #[cfg(debug_assertions)]
        let options: Options = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();

        let (main_to_worker_tx, main_to_worker_rx) = mpsc::channel();

        let write_verify_state = Arc::new(Mutex::new(None));

        Self::spawn_worker_thread(
            orc,
            runtime,
            cc.egui_ctx.clone(),
            main_to_worker_rx,
            write_verify_state.clone(),
        );

        let mut s = Self {
            options,
            main_to_worker_tx,
            write_verify_state,
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

    pub fn file_hash_ui(&mut self, ui: &mut egui::Ui) {
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

    pub fn image_ui(&mut self, ui: &mut egui::Ui) {
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

    pub fn target_disk_ui(&mut self, ui: &mut egui::Ui) {
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
                // FIXME: unwrapping! if any are missing we need to show this to user instead
                // TODO: remove this button and automaatically populate this ingoing values all are Some
                let write_verify_workflow = WriteVerifyWorkflow::new(
                    self.options.picked_image.clone().unwrap(),
                    self.options.detected_compression_format.unwrap(),
                    self.options.selected_write_target.clone().unwrap(),
                )
                .unwrap();

                self.main_to_worker_tx
                    .send(WorkerEvent::StartWrite(write_verify_workflow))
                    .unwrap()
            }
        });
    }
}

impl eframe::App for App {
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
            if let Some(write_verify_state) = &*self.write_verify_state.lock().unwrap() {
                match &*write_verify_state.borrow() {
                    WVState::Writing(writing) => {
                        ui.label("Burning");
                        ui.add(
                            ProgressBar::new(writing.approximate_ratio() as f32).show_percentage(),
                        );
                        if ui.button("Abort").clicked() {
                            self.main_to_worker_tx.send(WorkerEvent::Abort).unwrap();
                        }
                    }
                    WVState::Verifying {
                        total_write_bytes,
                        verify_hist,
                        ..
                    } => {
                        ui.label("Verifying");
                        let ratio =
                            verify_hist.bytes_encountered() as f32 / *total_write_bytes as f32;
                        ui.add(ProgressBar::new(ratio).show_percentage());
                        if ui.button("Abort").clicked() {
                            self.main_to_worker_tx.send(WorkerEvent::Abort).unwrap();
                        }
                    }
                    WVState::Finished {
                        finish_time,
                        result,
                        total_write_bytes,
                        ..
                    } => {
                        ui.label(format!(
                            "Finished in {finish_time:?} with result {result:?}"
                        ));
                        ui.label(format!("Total bytes written: {total_write_bytes}"));
                        if ui.button("Finish").clicked() {
                            self.main_to_worker_tx.send(WorkerEvent::Abort).unwrap();
                        }
                    }
                }
                return;
            }

            ui.label(RichText::new(env!("CARGO_PKG_NAME")).heading().size(26.));
            ui.label(env!("CARGO_PKG_DESCRIPTION"));

            ui.add_space(SECTION_SPACING * ui.spacing().item_spacing.y);

            self.image_ui(ui);

            ui.add_space(SECTION_SPACING * ui.spacing().item_spacing.y);

            ui.add_enabled_ui(self.options.picked_image.is_some(), |ui| {
                self.file_hash_ui(ui)
            });

            ui.add_space(SECTION_SPACING * ui.spacing().item_spacing.y);

            ui.add_enabled_ui(self.file_hash_is_verified_or_skipped(), |ui| {
                self.target_disk_ui(ui)
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
