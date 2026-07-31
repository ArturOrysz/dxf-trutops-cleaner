//! Minimal native GUI (egui): pick files → convert → log.

use crate::convert::convert_file;
use eframe::egui::{self, IconData};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

const DEFAULT_TOLERANCE: f64 = 0.1;

pub struct App {
    files: Vec<PathBuf>,
    log: String,
    busy: bool,
    rx: Option<Receiver<String>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            log: String::new(),
            busy: false,
            rx: None,
        }
    }
}

impl App {
    fn append_log(&mut self, line: impl AsRef<str>) {
        self.log.push_str(line.as_ref());
        self.log.push('\n');
    }

    fn pick_files(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("DXF / SVG", &["dxf", "svg"])
            .add_filter("DXF", &["dxf"])
            .add_filter("SVG", &["svg"])
            .pick_files()
        {
            for p in paths {
                if !self.files.iter().any(|x| x == &p) {
                    self.files.push(p);
                }
            }
        }
    }

    fn start_convert(&mut self) {
        if self.busy || self.files.is_empty() {
            return;
        }
        self.busy = true;
        let files = self.files.clone();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.append_log(format!(
            "---- Start: {} plik(ow), tol={DEFAULT_TOLERANCE} ----",
            files.len()
        ));
        thread::spawn(move || {
            for path in files {
                let _ = tx.send(format!("Wczytywanie: {}", path.display()));
                match convert_file(&path, DEFAULT_TOLERANCE, None) {
                    Ok(r) => {
                        let _ = tx.send(format!(
                            "OK: {} -> {} | kontury: {}",
                            r.before, r.after, r.contours
                        ));
                        if r.splines > 0 {
                            let _ = tx.send(format!(
                                "{} krzywych/sciazek: {} -> lukow: {}",
                                r.source, r.splines, r.arc_segments
                            ));
                        }
                        let _ = tx.send(format!("Zapisano: {}", r.output.display()));
                    }
                    Err(e) => {
                        let _ = tx.send(format!("BLAD: {} | {e}", path.display()));
                    }
                }
            }
            let _ = tx.send("__DONE__".into());
        });
    }

    fn poll_worker(&mut self, ctx: &egui::Context) {
        let mut messages = Vec::new();
        let mut done = false;
        if let Some(rx) = &self.rx {
            loop {
                match rx.try_recv() {
                    Ok(msg) if msg == "__DONE__" => {
                        done = true;
                        break;
                    }
                    Ok(msg) => messages.push(msg),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        done = true;
                        break;
                    }
                }
            }
        }
        for msg in messages {
            self.append_log(msg);
        }
        if done {
            self.busy = false;
            self.rx = None;
            self.append_log("Gotowe.");
        } else if self.busy {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_worker(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DXF / SVG → TruTops");
            ui.label("Bezier / polilinie / SPLINE → LINE + ARC");
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Wybierz DXF/SVG…"))
                    .clicked()
                {
                    self.pick_files();
                }
                if ui
                    .add_enabled(
                        !self.busy && !self.files.is_empty(),
                        egui::Button::new("Konwertuj"),
                    )
                    .clicked()
                {
                    self.start_convert();
                }
                if self.busy {
                    ui.spinner();
                }
            });

            if !self.files.is_empty() {
                ui.add_space(6.0);
                ui.label(format!("Wybrane: {} plik(ow)", self.files.len()));
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        for f in &self.files {
                            ui.monospace(
                                f.file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| f.display().to_string()),
                            );
                        }
                    });
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label("Log");
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.monospace(&self.log);
                });
        });
    }
}

fn load_icon() -> IconData {
    let png = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(png)
        .expect("icon.png")
        .into_rgba8();
    let (width, height) = image.dimensions();
    IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 420.0])
            .with_min_inner_size([400.0, 320.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "DXF Cleaner dla TruTops",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
