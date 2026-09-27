use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use egui::{Color32, CornerRadius, Margin, RichText, Stroke, Vec2};
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};

use crate::physics::HEAD_BLOCK;
use crate::solver::{self, Axis, Params, Progress, Solution, SortOptions};
use crate::theme as th;
use crate::widgets as w;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Form {
    pub power_sand: [String; 3],
    pub power_hammer: [String; 3],
    pub sand: [String; 3],
    pub hammer: [String; 3],
    pub tnt_amount: String,
    pub gametick_max: String,
    pub diff_gametick: String,
    pub limit_power: String,
    pub height_adjust_y: String,
    pub air_drag: String,
    pub gravity: String,
    pub axis: Axis,
    pub ratio_1x1: bool,
}

fn v3(v: [f64; 3]) -> [String; 3] {
    [v[0].to_string(), v[1].to_string(), v[2].to_string()]
}

impl Default for Form {
    fn default() -> Self {
        Self::from_params(&Params::default())
    }
}

impl Form {
    pub fn from_params(p: &Params) -> Self {
        Self {
            power_sand: v3(p.power_sand),
            power_hammer: v3(p.power_hammer),
            sand: v3(p.sand),
            hammer: v3(p.hammer),
            tnt_amount: p.tnt_amount.to_string(),
            gametick_max: p.gametick_max.to_string(),
            diff_gametick: p.diff_gametick.to_string(),
            limit_power: p.limit_power.to_string(),
            height_adjust_y: p.height_adjust_y.to_string(),
            air_drag: p.air_drag.to_string(),
            gravity: p.gravity.to_string(),
            axis: p.axis,
            ratio_1x1: p.ratio_1x1,
        }
    }

    pub fn to_params(&self) -> Result<Params, Vec<String>> {
        let mut errors = Vec::new();

        let mut vector = |name: &str, raw: &[String; 3]| -> [f64; 3] {
            let mut out = [0.0; 3];
            for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
                match parse_f64(&raw[i]) {
                    Some(v) => out[i] = v,
                    None => errors.push(format!("{name} {axis}")),
                }
            }
            out
        };

        let power_sand = vector("Power Sand", &self.power_sand);
        let power_hammer = vector("Power Hammer", &self.power_hammer);
        let sand = vector("Sand", &self.sand);
        let hammer = vector("Hammer", &self.hammer);

        let mut int_field = |name: &str, raw: &str| -> i64 {
            match parse_i64(raw) {
                Some(v) => v,
                None => {
                    errors.push(name.to_string());
                    0
                }
            }
        };
        let tnt_amount = int_field("Limit Power", &self.tnt_amount);
        let gametick_max = int_field("GT Hammer Max", &self.gametick_max);
        let diff_gametick = int_field("Diff Gametick", &self.diff_gametick);
        let limit_power = int_field("Limit Hammer", &self.limit_power);

        let mut float_field = |name: &str, raw: &str| -> f64 {
            match parse_f64(raw) {
                Some(v) => v,
                None => {
                    errors.push(name.to_string());
                    0.0
                }
            }
        };
        let height_adjust_y = float_field("Hauteur Adj", &self.height_adjust_y);
        let air_drag = float_field("Air drag", &self.air_drag);
        let gravity = float_field("Gravité", &self.gravity);

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(Params {
            power_sand,
            power_hammer,
            sand,
            hammer,
            tnt_amount,
            gametick_max,
            diff_gametick,
            limit_power,
            height_adjust_y,
            axis: self.axis,
            ratio_1x1: self.ratio_1x1,
            air_drag,
            gravity,
        })
    }
}

fn parse_f64(raw: &str) -> Option<f64> {
    let t = raw.trim().replace(',', ".");
    if t.is_empty() {
        return None;
    }
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn parse_i64(raw: &str) -> Option<i64> {
    parse_f64(raw).map(|v| v as i64)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Persisted {
    form: Form,
    sort: SortOptions,
    show_physics: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Results,
    Console,
}

struct LogLine {
    text: String,
    color: Color32,
}

enum Msg {
    Done {
        results: Vec<Solution>,
        elapsed: Duration,
        cancelled: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Idle,
    Running,
    Ok,
    Cancelled,
    Error,
}

pub struct MidairApp {
    form: Form,
    sort: SortOptions,
    show_physics: bool,

    results: Vec<Solution>,
    order: Vec<usize>,
    selected: Option<usize>,

    log: Vec<LogLine>,
    tab: Tab,

    progress: Arc<Progress>,
    rx: Option<Receiver<Msg>>,
    status: Status,
    elapsed: Option<Duration>,
    started: Option<Instant>,
}

impl MidairApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        th::install(&cc.egui_ctx);

        let saved: Persisted = cc
            .storage
            .and_then(|s| eframe::get_value::<Persisted>(s, eframe::APP_KEY))
            .unwrap_or_default();

        let mut app = Self {
            form: saved.form,
            sort: saved.sort,
            show_physics: saved.show_physics,
            results: Vec::new(),
            order: Vec::new(),
            selected: None,
            log: Vec::new(),
            tab: Tab::Results,
            progress: Arc::new(Progress::default()),
            rx: None,
            status: Status::Idle,
            elapsed: None,
            started: None,
        };
        app.info("Configure tes paramètres puis clique sur Lancer.");
        app
    }

    fn push_log(&mut self, text: impl Into<String>, color: Color32) {
        self.log.push(LogLine {
            text: text.into(),
            color,
        });
        if self.log.len() > 5000 {
            self.log.drain(0..1000);
        }
    }
    fn info(&mut self, t: impl Into<String>) {
        self.push_log(t, th::ACCENT);
    }
    fn ok(&mut self, t: impl Into<String>) {
        self.push_log(t, th::GREEN);
    }
    fn err(&mut self, t: impl Into<String>) {
        self.push_log(t, th::RED);
    }
    fn muted(&mut self, t: impl Into<String>) {
        self.push_log(t, th::MUTED);
    }

    fn running(&self) -> bool {
        self.status == Status::Running
    }

    fn start(&mut self, ctx: &egui::Context) {
        if self.running() {
            return;
        }
        let params = match self.form.to_params() {
            Ok(p) => p,
            Err(fields) => {
                self.tab = Tab::Console;
                self.err(format!("Paramètres invalides : {}", fields.join(", ")));
                self.status = Status::Error;
                return;
            }
        };

        if params.tnt_amount <= 0 || params.limit_power <= 0 {
            self.tab = Tab::Console;
            self.err("Limit Power et Limit Hammer doivent être supérieurs à 0.");
            self.status = Status::Error;
            return;
        }

        self.results.clear();
        self.order.clear();
        self.selected = None;
        self.muted("=".repeat(54));
        self.info(format!(
            "Calcul lancé — axe {}, ratio {}, {} puissances × {} hammer",
            params.axis.label(),
            if params.ratio_1x1 { "1x1" } else { "large" },
            params.tnt_amount,
            params.limit_power
        ));

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.status = Status::Running;
        self.started = Some(Instant::now());
        self.elapsed = None;

        let progress = self.progress.clone();
        progress.reset(params.tnt_amount.max(0) as usize);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let results = solver::solve(&params, &progress);
            let _ = tx.send(Msg::Done {
                results,
                elapsed: t0.elapsed(),
                cancelled: progress.is_cancelled(),
            });
            ctx.request_repaint();
        });
    }

    fn cancel(&mut self) {
        self.progress.cancel.store(true, Ordering::Relaxed);
    }

    fn poll(&mut self) {
        let Some(rx) = &self.rx else { return };
        match rx.try_recv() {
            Ok(Msg::Done {
                results,
                elapsed,
                cancelled,
            }) => {
                self.rx = None;
                self.elapsed = Some(elapsed);
                self.started = None;
                let n = results.len();
                self.results = results;
                self.resort();
                self.selected = self.order.first().copied();
                if cancelled {
                    self.status = Status::Cancelled;
                    self.push_log(
                        format!("Annulé — {n} résultat(s) partiel(s) en {:.2?}", elapsed),
                        th::YELLOW,
                    );
                } else {
                    self.status = Status::Ok;
                    self.ok(format!("Terminé — {n} résultat(s) en {:.2?}", elapsed));
                }
                self.muted("=".repeat(54));
                if n > 0 {
                    self.tab = Tab::Results;
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                self.started = None;
                self.status = Status::Error;
                self.tab = Tab::Console;
                self.err("Le calcul s'est interrompu de façon inattendue.");
            }
        }
    }

    fn resort(&mut self) {
        self.order = solver::sorted_indices(&self.results, &self.sort);
    }

    fn save_preset(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("preset-midair.json")
            .add_filter("JSON", &["json"])
            .save_file()
        else {
            return;
        };
        match serde_json::to_string_pretty(&self.form)
            .map_err(|e| e.to_string())
            .and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string()))
        {
            Ok(()) => self.ok(format!("Preset enregistré : {}", path.display())),
            Err(e) => self.err(format!("Échec de l'enregistrement : {e}")),
        }
    }

    fn load_preset(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<Form>(&s).map_err(|e| e.to_string()))
        {
            Ok(form) => {
                self.form = form;
                self.ok(format!("Preset chargé : {}", path.display()));
            }
            Err(e) => self.err(format!("Preset illisible : {e}")),
        }
    }

    fn export(&mut self, kind: ExportKind) {
        if self.results.is_empty() {
            self.err("Rien à exporter.");
            return;
        }
        let (name, filter, ext) = match kind {
            ExportKind::Csv => ("resultats-midair.csv", "CSV", "csv"),
            ExportKind::Json => ("resultats-midair.json", "JSON", "json"),
            ExportKind::Text => ("resultats-midair.txt", "Texte", "txt"),
        };
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(name)
            .add_filter(filter, &[ext])
            .save_file()
        else {
            return;
        };

        let ordered: Vec<&Solution> = self.order.iter().map(|&i| &self.results[i]).collect();
        let body = match kind {
            ExportKind::Csv => to_csv(&ordered),
            ExportKind::Json => serde_json::to_string_pretty(&ordered).unwrap_or_default(),
            ExportKind::Text => ordered
                .iter()
                .enumerate()
                .map(|(i, r)| format!("-- #{}\n{}", i + 1, report(r)))
                .collect::<Vec<_>>()
                .join("\n"),
        };
        match std::fs::write(&path, body) {
            Ok(()) => self.ok(format!(
                "{} résultat(s) exporté(s) vers {}",
                ordered.len(),
                path.display()
            )),
            Err(e) => self.err(format!("Échec de l'export : {e}")),
        }
    }
}

#[derive(Clone, Copy)]
enum ExportKind {
    Csv,
    Json,
    Text,
}

impl eframe::App for MidairApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let p = Persisted {
            form: self.form.clone(),
            sort: self.sort,
            show_physics: self.show_physics,
        };
        eframe::set_value(storage, eframe::APP_KEY, &p);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.poll();
        if self.running() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }

        let (run_shortcut, cancel_shortcut) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if run_shortcut && !self.running() {
            self.start(&ctx);
        }
        if cancel_shortcut && self.running() {
            self.cancel();
        }

        self.header(ui);
        self.action_bar(ui);
        self.detail_panel(ui);
        self.params_panel(ui);
        self.central(ui);
    }
}

impl MidairApp {
    fn header(&mut self, ui: &mut egui::Ui) {
        let frame = egui::Frame::new()
            .fill(th::SURFACE)
            .inner_margin(Margin::symmetric(20, 0));

        egui::Panel::top("header")
            .frame(frame)
            .exact_size(58.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(6), th::ACCENT);
                    ui.add_space(4.0);
                    ui.label(RichText::new("SCRIPT").size(17.0).strong().color(th::TEXT));
                    ui.add_space(-4.0);
                    ui.label(
                        RichText::new("MIDAIR")
                            .size(17.0)
                            .strong()
                            .color(th::ACCENT),
                    );

                    let h = ui.available_height();
                    w::right_aligned(ui, h, |ui| {
                        let (dot, text) = self.status_text();
                        w::pill(ui, dot, &text);

                        if self.running() {
                            ui.add_space(10.0);
                            ui.add(
                                egui::ProgressBar::new(self.progress.fraction())
                                    .desired_width(180.0)
                                    .desired_height(8.0)
                                    .corner_radius(CornerRadius::same(4))
                                    .fill(th::ACCENT),
                            );
                        } else if let Some(d) = self.elapsed {
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new(format!("{:.2?}", d))
                                    .size(10.5)
                                    .monospace()
                                    .color(th::MUTED),
                            );
                        }
                    });
                });
            });
    }

    fn status_text(&self) -> (Color32, String) {
        match self.status {
            Status::Idle => (th::MUTED, "Prêt".into()),
            Status::Running => {
                let pct = (self.progress.fraction() * 100.0).round() as i32;
                let found = self.progress.found.load(Ordering::Relaxed);
                let secs = self
                    .started
                    .map(|s| s.elapsed().as_secs_f32())
                    .unwrap_or(0.0);
                (
                    th::ACCENT,
                    format!("Calcul {pct}% · {found} trouvé(s) · {secs:.0}s"),
                )
            }
            Status::Ok => (
                th::GREEN,
                format!("Prêt · {} résultats", self.results.len()),
            ),
            Status::Cancelled => (
                th::YELLOW,
                format!("Annulé · {} résultats", self.results.len()),
            ),
            Status::Error => (th::RED, "Erreur".into()),
        }
    }

    fn params_panel(&mut self, ui: &mut egui::Ui) {
        let frame = egui::Frame::new()
            .fill(th::BG)
            .inner_margin(Margin::symmetric(14, 12));

        egui::Panel::left("params")
            .frame(frame)
            .resizable(true)
            .default_size(424.0)
            .size_range(egui::Rangef::new(360.0, 620.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.barrels_card(ui);
                        ui.add_space(10.0);
                        self.settings_card(ui);
                        ui.add_space(10.0);
                        self.preview_card(ui);
                        ui.add_space(10.0);
                        self.preset_row(ui);
                        ui.add_space(8.0);
                    });
            });
    }

    fn barrels_card(&mut self, ui: &mut egui::Ui) {
        th::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            w::section_title(ui, "Barrels");
            ui.label(
                RichText::new(
                    "Astuce : colle « x y z » dans le champ X, il se répartit tout seul.",
                )
                .size(9.5)
                .color(th::MUTED),
            );
            ui.add_space(6.0);

            vector_row(ui, "Power Sand", &mut self.form.power_sand);
            vector_row(ui, "Power Hammer", &mut self.form.power_hammer);
            ui.horizontal(|ui| {
                if small_link(ui, "Copier Power Sand vers Power Hammer") {
                    self.form.power_hammer = self.form.power_sand.clone();
                }
            });
            ui.add_space(6.0);
            vector_row(ui, "Sand", &mut self.form.sand);
            vector_row(ui, "Hammer", &mut self.form.hammer);
            ui.horizontal(|ui| {
                if small_link(ui, "Copier Sand vers Hammer") {
                    self.form.hammer = self.form.sand.clone();
                }
            });
        });
    }

    fn settings_card(&mut self, ui: &mut egui::Ui) {
        th::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            w::section_title(ui, "Paramètres");

            let full = ui.available_width();
            let half = (full - 8.0) / 2.0;

            ui.horizontal(|ui| {
                scalar_field(ui, "Limit Power", &mut self.form.tnt_amount, half - 16.0);
                scalar_field(
                    ui,
                    "GT Hammer Max",
                    &mut self.form.gametick_max,
                    half - 16.0,
                );
            });
            ui.horizontal(|ui| {
                scalar_field(
                    ui,
                    "Diff Gametick",
                    &mut self.form.diff_gametick,
                    half - 16.0,
                );
                scalar_field(ui, "Limit Hammer", &mut self.form.limit_power, half - 16.0);
            });

            ui.add_space(4.0);
            ui.vertical(|ui| {
                w::micro_label(ui, "Hauteur Adj");
                ui.horizontal(|ui| {
                    let valid = parse_f64(&self.form.height_adjust_y).is_some();
                    w::num_field(ui, &mut self.form.height_adjust_y, full - 92.0, valid);
                    if ui
                        .add(
                            egui::Button::new(RichText::new("+ head").size(10.0).color(th::TEXT2))
                                .fill(th::SUBTLE)
                                .corner_radius(CornerRadius::same(7)),
                        )
                        .on_hover_text(format!("Ajoute le head block ({HEAD_BLOCK})"))
                        .clicked()
                        && let Some(v) = parse_f64(&self.form.height_adjust_y)
                    {
                        self.form.height_adjust_y = (v + HEAD_BLOCK).to_string();
                    }
                });
            });

            ui.add_space(8.0);
            w::checkbox(ui, &mut self.form.ratio_1x1, "Ratio 1x1")
                .on_hover_text("Coché : fenêtre [0.49, 0.51]. Décoché : [0.01, 0.99].");

            ui.add_space(8.0);
            w::micro_label(ui, "Axis");
            w::segmented(
                ui,
                &mut self.form.axis,
                &[(Axis::X, "x"), (Axis::Z, "z")],
                full,
            );

            ui.add_space(8.0);
            let arrow = if self.show_physics { "[-]" } else { "[+]" };
            if small_link(ui, &format!("{arrow}  Constantes physiques")) {
                self.show_physics = !self.show_physics;
            }
            if self.show_physics {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    scalar_field(ui, "Air drag", &mut self.form.air_drag, half - 16.0);
                    scalar_field(ui, "Gravité", &mut self.form.gravity, half - 16.0);
                });
                if small_link(ui, "Valeurs par défaut") {
                    let d = Params::default();
                    self.form.air_drag = d.air_drag.to_string();
                    self.form.gravity = d.gravity.to_string();
                }
            }
        });
    }

    fn preview_card(&mut self, ui: &mut egui::Ui) {
        th::card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            w::section_title(ui, "Efficacités");

            match self.form.to_params() {
                Ok(p) => {
                    let (sand, hammer) = solver::barrels(&p);
                    for (name, b, color) in
                        [("Sand", sand, th::YELLOW), ("Hammer", hammer, th::GREEN)]
                    {
                        ui.label(RichText::new(name).size(10.5).strong().color(color));
                        w::kv(
                            ui,
                            "eff",
                            format!("{:+.6}  {:+.6}  {:+.6}", b.eff_x, b.eff_y, b.eff_z),
                            th::TEXT2,
                        );
                        w::kv(
                            ui,
                            "dist",
                            format!("{:.6}", b.dist_eff),
                            if b.dist_eff <= 0.0 {
                                th::RED
                            } else {
                                th::TEXT2
                            },
                        );
                        if b.dist_eff <= 0.0 {
                            ui.label(
                                RichText::new(
                                    "Attention : distance >= 8, efficacité nulle ou négative",
                                )
                                .size(9.5)
                                .color(th::RED),
                            );
                        }
                        ui.add_space(4.0);
                    }
                }
                Err(fields) => {
                    ui.label(
                        RichText::new(format!("Champs invalides : {}", fields.join(", ")))
                            .size(10.5)
                            .color(th::RED),
                    );
                }
            }
        });
    }

    fn preset_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let wdt = (ui.available_width() - 16.0) / 3.0;
            if w::button(ui, "Charger", false, Vec2::new(wdt, 30.0)).clicked() {
                self.load_preset();
            }
            if w::button(ui, "Sauver", false, Vec2::new(wdt, 30.0)).clicked() {
                self.save_preset();
            }
            if w::button(ui, "Défauts", false, Vec2::new(wdt, 30.0))
                .on_hover_text("Rétablit les paramètres d'origine")
                .clicked()
            {
                self.form = Form::default();
                self.muted("Paramètres réinitialisés.");
            }
        });
    }

    fn action_bar(&mut self, ui: &mut egui::Ui) {
        let frame = egui::Frame::new()
            .fill(th::SURFACE)
            .stroke(Stroke::new(1.0, th::BORDER))
            .inner_margin(Margin::symmetric(20, 10));

        egui::Panel::bottom("actions")
            .frame(frame)
            .exact_size(126.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        w::section_title(ui, "Tri des résultats");
                        let mut changed = false;
                        ui.horizontal(|ui| {
                            changed |= w::checkbox(
                                ui,
                                &mut self.sort.by_dispensers,
                                "Moins de dispensers",
                            )
                            .changed();
                            ui.add_space(10.0);
                            changed |=
                                w::checkbox(ui, &mut self.sort.by_gap, "Moins d'écart").changed();
                        });
                        ui.horizontal(|ui| {
                            changed |= w::checkbox(
                                ui,
                                &mut self.sort.by_sand_hammer_gap,
                                "Écart Sand/Hammer",
                            )
                            .changed();
                            ui.add_space(10.0);
                            changed |=
                                w::checkbox(ui, &mut self.sort.by_distance, "Distance").changed();
                            ui.add_enabled_ui(self.sort.by_distance, |ui| {
                                changed |= ui
                                    .radio_value(&mut self.sort.distance_desc, true, "max")
                                    .changed();
                                changed |= ui
                                    .radio_value(&mut self.sort.distance_desc, false, "min")
                                    .changed();
                            });
                        });
                        if changed {
                            self.resort();
                        }
                    });

                    let h = ui.available_height();
                    w::right_aligned(ui, h, |ui| {
                        if self.running() {
                            if w::button(ui, "ANNULER", false, Vec2::new(168.0, 50.0)).clicked() {
                                self.cancel();
                            }
                        } else if w::button(ui, "LANCER", true, Vec2::new(168.0, 50.0))
                            .on_hover_text("Ctrl + Entrée")
                            .clicked()
                        {
                            let ctx = ui.ctx().clone();
                            self.start(&ctx);
                        }

                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.add_enabled_ui(!self.results.is_empty(), |ui| {
                                ui.horizontal(|ui| {
                                    if w::button(ui, "CSV", false, Vec2::new(54.0, 23.0)).clicked()
                                    {
                                        self.export(ExportKind::Csv);
                                    }
                                    if w::button(ui, "JSON", false, Vec2::new(54.0, 23.0)).clicked()
                                    {
                                        self.export(ExportKind::Json);
                                    }
                                    if w::button(ui, "TXT", false, Vec2::new(54.0, 23.0))
                                        .on_hover_text("Format texte du script d'origine")
                                        .clicked()
                                    {
                                        self.export(ExportKind::Text);
                                    }
                                });
                            });
                            if w::button(ui, "Effacer la console", false, Vec2::new(174.0, 23.0))
                                .clicked()
                            {
                                self.log.clear();
                            }
                        });
                    });
                });
            });
    }

    fn detail_panel(&mut self, ui: &mut egui::Ui) {
        let Some(sel) = self.selected else { return };
        if self.tab != Tab::Results || sel >= self.results.len() {
            return;
        }
        let r = self.results[sel].clone();

        let frame = egui::Frame::new().fill(th::BG).inner_margin(Margin {
            left: 14,
            right: 14,
            top: 0,
            bottom: 12,
        });

        egui::Panel::bottom("detail")
            .frame(frame)
            .exact_size(196.0)
            .show(ui, |ui| {
                th::card().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        w::section_title(ui, "Détail de la solution");
                        w::right_aligned(ui, 24.0, |ui| {
                            if w::button(ui, "Copier", false, Vec2::new(96.0, 24.0)).clicked() {
                                ui.ctx().copy_text(report(&r));
                            }
                        });
                    });
                    ui.columns(3, |cols| {
                        detail_group(
                            &mut cols[0],
                            "SAND",
                            th::YELLOW,
                            &[
                                ("Power", r.sand_power.to_string()),
                                ("Position", r.sand_position.to_string()),
                                ("Distance", r.sand_distance.to_string()),
                                ("Gametick", r.sand_gametick.to_string()),
                                ("Velocity", r.sand_velocity.to_string()),
                                ("Position Y", r.sand_position_y.to_string()),
                            ],
                        );
                        detail_group(
                            &mut cols[1],
                            "HAMMER",
                            th::GREEN,
                            &[
                                ("Power", r.hammer_power.to_string()),
                                ("Position", r.hammer_position.to_string()),
                                ("Gametick", r.hammer_gametick.to_string()),
                                ("Position Y", r.hammer_position_y.to_string()),
                            ],
                        );
                        detail_group(
                            &mut cols[2],
                            "RATIO",
                            th::RED,
                            &[
                                ("Hammer", r.ratio_hammer.to_string()),
                                ("Velocity", r.ratio_velocity.to_string()),
                                ("Y-velocity", r.final_velocity_y.to_string()),
                                ("Dispensers", r.total_dispensers.to_string()),
                                ("Écart moyen", format!("{:.2}", r.gap_avg)),
                                ("Écart S/H", r.sand_hammer_gap.to_string()),
                            ],
                        );
                    });
                });
            });
    }

    fn central(&mut self, ui: &mut egui::Ui) {
        let frame = egui::Frame::new()
            .fill(th::BG)
            .inner_margin(Margin::symmetric(14, 12));

        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            ui.horizontal(|ui| {
                w::segmented(
                    ui,
                    &mut self.tab,
                    &[(Tab::Results, "Résultats"), (Tab::Console, "Console")],
                    220.0,
                );
                w::right_aligned(ui, 28.0, |ui| {
                    if self.tab == Tab::Results && !self.results.is_empty() {
                        ui.label(
                            RichText::new(format!("{} solution(s)", self.results.len()))
                                .size(10.5)
                                .color(th::MUTED),
                        );
                    }
                });
            });
            ui.add_space(8.0);

            match self.tab {
                Tab::Results => self.results_table(ui),
                Tab::Console => self.console(ui),
            }
        });
    }

    fn results_table(&mut self, ui: &mut egui::Ui) {
        let inner = ui.available_size() - Vec2::splat(18.0);

        if self.results.is_empty() {
            th::sunken().show(ui, |ui| {
                ui.set_min_size(inner);
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 30.0);
                    ui.label(RichText::new("Aucun résultat").size(14.0).color(th::MUTED));
                    ui.label(
                        RichText::new("Règle tes barrels à gauche puis lance le calcul.")
                            .size(11.0)
                            .color(th::BORDER2),
                    );
                });
            });
            return;
        }

        let order = self.order.clone();
        let mut new_selection = None;

        th::sunken().show(ui, |ui| {
            ui.set_min_size(inner);
            let header = |ui: &mut egui::Ui, text: &str, color: Color32| {
                ui.label(RichText::new(text).size(9.5).strong().color(color));
            };

            TableBuilder::new(ui)
                .max_scroll_height(inner.y - 30.0)
                .striped(true)
                .sense(egui::Sense::click())
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(42.0))
                .column(Column::initial(66.0).at_least(50.0))
                .column(Column::initial(76.0).at_least(56.0))
                .column(Column::initial(66.0).at_least(50.0))
                .column(Column::initial(104.0).at_least(70.0))
                .column(Column::exact(40.0))
                .column(Column::initial(104.0).at_least(70.0))
                .column(Column::initial(104.0).at_least(70.0))
                .column(Column::initial(70.0).at_least(56.0))
                .column(Column::remainder().at_least(60.0))
                .header(26.0, |mut row| {
                    row.col(|ui| header(ui, "#", th::MUTED));
                    row.col(|ui| header(ui, "SAND PW", th::YELLOW));
                    row.col(|ui| header(ui, "HAMMER PW", th::GREEN));
                    row.col(|ui| header(ui, "RATIO", th::RED));
                    row.col(|ui| header(ui, "DISTANCE", th::BLUE));
                    row.col(|ui| header(ui, "GT", th::MUTED));
                    row.col(|ui| header(ui, "R.VELOCITY", th::MUTED));
                    row.col(|ui| header(ui, "Y-VELOCITY", th::MUTED));
                    row.col(|ui| header(ui, "DISPENSERS", th::MUTED));
                    row.col(|ui| header(ui, "ÉCART", th::MUTED));
                })
                .body(|body| {
                    body.rows(24.0, order.len(), |mut row| {
                        let rank = row.index();
                        let idx = order[rank];
                        let r = &self.results[idx];
                        row.set_selected(self.selected == Some(idx));

                        let cell = |ui: &mut egui::Ui, text: String, color: Color32| {
                            ui.label(RichText::new(text).size(11.0).monospace().color(color));
                        };

                        row.col(|ui| cell(ui, format!("{}", rank + 1), th::MUTED));
                        row.col(|ui| cell(ui, r.sand_power.to_string(), th::YELLOW));
                        row.col(|ui| cell(ui, r.hammer_power.to_string(), th::GREEN));
                        row.col(|ui| cell(ui, r.ratio_hammer.to_string(), th::RED));
                        row.col(|ui| cell(ui, format!("{:.4}", r.sand_distance), th::BLUE));
                        row.col(|ui| cell(ui, r.sand_gametick.to_string(), th::TEXT2));
                        row.col(|ui| cell(ui, format!("{:.3e}", r.ratio_velocity), th::TEXT2));
                        row.col(|ui| cell(ui, format!("{:.4}", r.final_velocity_y), th::TEXT2));
                        row.col(|ui| cell(ui, r.total_dispensers.to_string(), th::TEXT2));
                        row.col(|ui| cell(ui, format!("{:.2}", r.gap_avg), th::TEXT2));

                        if row.response().clicked() {
                            new_selection = Some(idx);
                        }
                    });
                });
        });

        if let Some(idx) = new_selection {
            self.selected = Some(idx);
        }
    }

    fn console(&mut self, ui: &mut egui::Ui) {
        let inner = ui.available_size() - Vec2::splat(18.0);
        th::sunken().show(ui, |ui| {
            ui.set_min_size(inner);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for line in &self.log {
                        ui.label(
                            RichText::new(&line.text)
                                .size(11.5)
                                .monospace()
                                .color(line.color),
                        );
                    }
                });
        });
    }
}

fn vector_row(ui: &mut egui::Ui, label: &str, values: &mut [String; 3]) {
    ui.add_space(2.0);
    ui.label(RichText::new(label).size(11.0).strong().color(th::TEXT));
    ui.add_space(3.0);
    let field_w = ui.available_width() - 34.0;
    for i in 0..3 {
        ui.horizontal(|ui| {
            ui.add_sized(
                Vec2::new(12.0, 22.0),
                egui::Label::new(
                    RichText::new(["X", "Y", "Z"][i])
                        .size(9.5)
                        .strong()
                        .color(th::MUTED),
                ),
            );
            let valid = parse_f64(&values[i]).is_some();
            if w::num_field(ui, &mut values[i], field_w, valid) && i == 0 {
                split_paste(values);
            }
        });
    }
    ui.add_space(6.0);
}

fn split_paste(values: &mut [String; 3]) {
    let raw = values[0].clone();
    let parts: Vec<&str> = raw
        .split([' ', '\t', ';', '/'])
        .filter(|s| !s.trim().is_empty())
        .collect();
    if parts.len() == 3 && parts.iter().all(|p| parse_f64(p).is_some()) {
        for (slot, part) in values.iter_mut().zip(parts) {
            *slot = part.trim().to_string();
        }
    }
}

fn scalar_field(ui: &mut egui::Ui, label: &str, value: &mut String, width: f32) {
    ui.vertical(|ui| {
        w::micro_label(ui, label);
        let valid = parse_f64(value).is_some();
        w::num_field(ui, value, width, valid);
    });
}

fn small_link(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add(egui::Label::new(
        RichText::new(text).size(10.0).color(th::TEXT2),
    ))
    .on_hover_cursor(egui::CursorIcon::PointingHand)
    .interact(egui::Sense::click())
    .clicked()
}

fn detail_group(ui: &mut egui::Ui, title: &str, color: Color32, rows: &[(&str, String)]) {
    ui.label(RichText::new(title).size(10.0).strong().color(color));
    ui.add_space(2.0);
    ui.spacing_mut().item_spacing.y = 2.0;
    for (k, v) in rows {
        w::kv(ui, k, v.clone(), th::TEXT);
    }
}

fn report(r: &Solution) -> String {
    format!(
        "SAND\nPower {} | Position {} | Distance {} | Gametick {} | Velocity {} | Position Y {}\n\
         HAMMER\nPower {} | Position {} | Gametick {} | Position Y {}\n\
         Hammer {} | velocity {} | Y-velocity {}\n",
        r.sand_power,
        r.sand_position,
        r.sand_distance,
        r.sand_gametick,
        r.sand_velocity,
        r.sand_position_y,
        r.hammer_power,
        r.hammer_position,
        r.hammer_gametick,
        r.hammer_position_y,
        r.ratio_hammer,
        r.ratio_velocity,
        r.final_velocity_y,
    )
}

fn to_csv(rows: &[&Solution]) -> String {
    let mut out = String::from(
        "rang,sand_power,sand_position,sand_distance,sand_gametick,sand_velocity,sand_position_y,\
         hammer_power,hammer_position,hammer_gametick,hammer_position_y,\
         ratio_hammer,ratio_velocity,final_velocity_y,total_dispensers,gap_avg,sand_hammer_gap\n",
    );
    for (i, r) in rows.iter().enumerate() {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            i + 1,
            r.sand_power,
            r.sand_position,
            r.sand_distance,
            r.sand_gametick,
            r.sand_velocity,
            r.sand_position_y,
            r.hammer_power,
            r.hammer_position,
            r.hammer_gametick,
            r.hammer_position_y,
            r.ratio_hammer,
            r.ratio_velocity,
            r.final_velocity_y,
            r.total_dispensers,
            r.gap_avg,
            r.sand_hammer_gap,
        ));
    }
    out
}
