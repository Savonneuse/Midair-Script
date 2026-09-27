use std::sync::Arc;

use crate::app::Form;
use crate::solver::{self, Params, Progress};

pub fn run(args: &[String]) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("usage: ScriptMidair --cli <preset.json> [--json]");
        return 2;
    };
    let as_json = args.iter().any(|a| a == "--json");

    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lecture impossible de {path} : {e}");
            return 1;
        }
    };

    let params: Params = match serde_json::from_str::<Form>(&raw) {
        Ok(form) => match form.to_params() {
            Ok(p) => p,
            Err(fields) => {
                eprintln!("champs invalides : {}", fields.join(", "));
                return 1;
            }
        },
        Err(_) => match serde_json::from_str::<Params>(&raw) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("JSON illisible : {e}");
                return 1;
            }
        },
    };

    let progress = Arc::new(Progress::default());
    let results = solver::solve(&params, &progress);

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
    } else {
        for r in &results {
            println!("SAND");
            println!(
                "Power {} | Position {} | Distance {} | Gametick {} | Velocity {} | Position Y {}",
                r.sand_power,
                r.sand_position,
                r.sand_distance,
                r.sand_gametick,
                r.sand_velocity,
                r.sand_position_y
            );
            println!("HAMMER");
            println!(
                "Power {} | Position {} | Gametick {} | Position Y {}",
                r.hammer_power, r.hammer_position, r.hammer_gametick, r.hammer_position_y
            );
            println!(
                "Hammer {} | velocity {} | Y-velocity {}\n",
                r.ratio_hammer, r.ratio_velocity, r.final_velocity_y
            );
        }
        eprintln!("{} resultat(s)", results.len());
    }
    0
}
