#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod console;
mod physics;
mod solver;
mod theme;
mod widgets;

use app::MidairApp;

fn window_icon() -> Option<egui::IconData> {
    const ICO: &[u8] = include_bytes!("../assets/orange.ico");
    let img = image::load_from_memory_with_format(ICO, image::ImageFormat::Ico).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--cli") {
        console::attach_to_parent();
        std::process::exit(cli::run(&args[1..]));
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Script Midair")
        .with_inner_size([1320.0, 860.0])
        .with_min_inner_size([1060.0, 660.0]);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Script Midair",
        options,
        Box::new(|cc| Ok(Box::new(MidairApp::new(cc)))),
    )
}
