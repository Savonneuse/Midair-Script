use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::physics::{AIR_DRAG, Barrel, GRAVITY, barrel_calc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis {
    X,
    Z,
}

impl Axis {
    pub fn label(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Z => "z",
        }
    }
}

pub type Vec3 = [f64; 3];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Params {
    pub power_sand: Vec3,
    pub power_hammer: Vec3,
    pub sand: Vec3,
    pub hammer: Vec3,
    pub tnt_amount: i64,
    pub gametick_max: i64,
    pub diff_gametick: i64,
    pub limit_power: i64,
    pub height_adjust_y: f64,
    pub axis: Axis,
    pub ratio_1x1: bool,
    pub air_drag: f64,
    pub gravity: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            power_sand: [368_092.509_999_990_46, 6.0, -396_871.509_999_990_46],
            power_hammer: [368_092.509_999_990_46, 6.0, -396_871.509_999_990_46],
            sand: [
                368_092.884_999_990_46,
                6.519_999_980_926_514,
                -396_870.509_999_990_46,
            ],
            hammer: [
                368_092.884_999_990_46,
                6.519_999_980_926_512,
                -396_870.490_000_009_54,
            ],
            tnt_amount: 200,
            gametick_max: 24,
            diff_gametick: 4,
            limit_power: 5000,
            height_adjust_y: 255.0,
            axis: Axis::Z,
            ratio_1x1: true,
            air_drag: AIR_DRAG,
            gravity: GRAVITY,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Solution {
    pub sand_power: i64,
    pub sand_position: f64,
    pub sand_distance: f64,
    pub sand_gametick: i64,
    pub sand_velocity: f64,
    pub sand_position_y: f64,
    pub hammer_power: i64,
    pub hammer_position: f64,
    pub hammer_gametick: i64,
    pub hammer_position_y: f64,
    pub ratio_hammer: i64,
    pub ratio_velocity: f64,
    pub final_velocity_y: f64,
    pub total_dispensers: i64,
    pub gap_avg: f64,
    pub sand_hammer_gap: i64,
}

#[derive(Debug, Default)]
pub struct Progress {
    pub total: AtomicUsize,
    pub done: AtomicUsize,
    pub found: AtomicUsize,
    pub cancel: AtomicBool,
}

impl Progress {
    pub fn reset(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.done.store(0, Ordering::Relaxed);
        self.found.store(0, Ordering::Relaxed);
        self.cancel.store(false, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn fraction(&self) -> f32 {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        (self.done.load(Ordering::Relaxed) as f32 / total as f32).clamp(0.0, 1.0)
    }
}

struct Ctx {
    sand_eff: f64,
    hammer_eff: f64,
    sand_dist_eff: f64,
    hammer_dist_eff: f64,
    proj_sand: f64,
    proj_hammer: f64,
    height_adjust_y: f64,
    gametick_max: i64,
    diff_gametick: i64,
    limit_power: i64,
    axis: Axis,
    ratio_low: f64,
    ratio_high: f64,
    air_drag: f64,
    gravity: f64,
}

pub fn barrels(p: &Params) -> (Barrel, Barrel) {
    let sand = barrel_calc(
        p.power_sand[0],
        p.power_sand[1],
        p.power_sand[2],
        p.sand[0],
        p.sand[1],
        p.sand[2],
    );
    let hammer = barrel_calc(
        p.power_hammer[0],
        p.power_hammer[1],
        p.power_hammer[2],
        p.hammer[0],
        p.hammer[1],
        p.hammer[2],
    );
    (sand, hammer)
}

pub fn solve(p: &Params, progress: &Arc<Progress>) -> Vec<Solution> {
    let (sand_b, hammer_b) = barrels(p);
    let (ratio_low, ratio_high) = if p.ratio_1x1 {
        (0.49, 0.51)
    } else {
        (0.01, 0.99)
    };

    let idx = match p.axis {
        Axis::X => 0,
        Axis::Z => 2,
    };

    let ctx = Ctx {
        sand_eff: sand_b.eff_on(p.axis),
        hammer_eff: hammer_b.eff_on(p.axis),
        sand_dist_eff: sand_b.dist_eff,
        hammer_dist_eff: hammer_b.dist_eff,
        proj_sand: p.sand[idx],
        proj_hammer: p.hammer[idx],
        height_adjust_y: p.height_adjust_y,
        gametick_max: p.gametick_max,
        diff_gametick: p.diff_gametick,
        limit_power: p.limit_power,
        axis: p.axis,
        ratio_low,
        ratio_high,
        air_drag: p.air_drag,
        gravity: p.gravity,
    };

    let count = p.tnt_amount.max(0);
    progress.reset(count as usize);

    let chunks: Vec<Vec<Solution>> = (0..count)
        .into_par_iter()
        .map(|i| {
            let out = if progress.is_cancelled() {
                Vec::new()
            } else {
                sweep_sand_power(i, &ctx, progress)
            };
            progress.done.fetch_add(1, Ordering::Relaxed);
            progress.found.fetch_add(out.len(), Ordering::Relaxed);
            out
        })
        .collect();

    chunks.into_iter().flatten().collect()
}

fn sweep_sand_power(i: i64, c: &Ctx, progress: &Arc<Progress>) -> Vec<Solution> {
    let mut out = Vec::new();

    let mut range_sand_value = c.sand_eff * c.sand_dist_eff * i as f64;
    let mut range_sand_total = range_sand_value + c.proj_sand;
    let mut range_sand_value_y = 0.0_f64;
    let mut range_sand_total_y = c.height_adjust_y;

    for j in 2..c.gametick_max {
        range_sand_value *= c.air_drag;
        range_sand_total += range_sand_value;
        let velocity = range_sand_value * c.air_drag;

        range_sand_value_y -= c.gravity;
        range_sand_total_y += range_sand_value_y;
        range_sand_value_y *= c.air_drag;

        let frac = range_sand_total.rem_euclid(1.0);
        if !(c.ratio_low <= frac && frac <= c.ratio_high) {
            continue;
        }

        for k in i..c.limit_power {
            if k % 4096 == 0 && progress.is_cancelled() {
                return out;
            }

            let mut range_hammer_value = c.hammer_eff * c.hammer_dist_eff * k as f64;
            let mut range_hammer_total = range_hammer_value + c.proj_hammer;
            let mut range_hammer_value_y = 0.0_f64;
            let mut range_hammer_total_y = c.height_adjust_y;

            let mut l: i64 = 0;
            for li in 1..=(j - c.diff_gametick) {
                range_hammer_value *= c.air_drag;
                range_hammer_total += range_hammer_value;

                range_hammer_value_y -= c.gravity;
                range_hammer_total_y += range_hammer_value_y;
                range_hammer_value_y *= c.air_drag;
                l = li;
            }

            let behind = if c.sand_eff > 0.0 {
                range_hammer_total - range_sand_total
            } else {
                range_sand_total - range_hammer_total
            };

            if !(0.0..=4.0).contains(&behind) {
                continue;
            }

            let ratio = match c.axis {
                Axis::Z => barrel_calc(
                    1.0,
                    range_hammer_total_y,
                    range_hammer_total,
                    1.0,
                    range_sand_total_y,
                    range_sand_total,
                ),
                Axis::X => barrel_calc(
                    range_hammer_total,
                    range_hammer_total_y,
                    1.0,
                    range_sand_total,
                    range_sand_total_y,
                    1.0,
                ),
            };

            let step = ratio.eff_on(c.axis) * ratio.dist_eff;

            let mut best_hammer: Option<i64> = None;
            let mut best_diff = f64::INFINITY;
            for m in 0..c.limit_power {
                let diff = (velocity + step * m as f64).abs();
                if diff < best_diff {
                    best_diff = diff;
                    best_hammer = Some(m);
                } else if diff > best_diff {
                    break;
                }
            }
            let Some(best_hammer) = best_hammer else {
                continue;
            };

            let velocity_sand_y = ratio.eff_y * ratio.dist_eff * best_hammer as f64;
            let velocity_final_y = range_sand_total_y + velocity_sand_y + range_sand_value_y;

            if velocity_final_y < 1.0 {
                out.push(Solution {
                    sand_power: i,
                    sand_position: range_sand_total,
                    sand_distance: range_sand_total - c.proj_sand,
                    sand_gametick: j,
                    sand_velocity: velocity,
                    sand_position_y: range_sand_total_y,
                    hammer_power: k,
                    hammer_position: range_hammer_total,
                    hammer_gametick: l,
                    hammer_position_y: range_hammer_total_y,
                    ratio_hammer: best_hammer,
                    ratio_velocity: best_diff,
                    final_velocity_y: velocity_final_y,
                    total_dispensers: i + k + best_hammer,
                    gap_avg: ((i - k).abs() + (i - best_hammer).abs() + (k - best_hammer).abs())
                        as f64
                        / 3.0,
                    sand_hammer_gap: (i - k).abs(),
                });
            }
        }
    }

    out
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SortOptions {
    pub by_dispensers: bool,
    pub by_gap: bool,
    pub by_sand_hammer_gap: bool,
    pub by_distance: bool,
    pub distance_desc: bool,
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            by_dispensers: false,
            by_gap: false,
            by_sand_hammer_gap: false,
            by_distance: false,
            distance_desc: true,
        }
    }
}

impl SortOptions {
    pub fn any(&self) -> bool {
        self.by_dispensers || self.by_gap || self.by_sand_hammer_gap || self.by_distance
    }
}

fn rank(values: &[f64], reverse: bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    if reverse {
        order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    } else {
        order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    }
    let mut ranks = vec![0usize; values.len()];
    for (r, &idx) in order.iter().enumerate() {
        ranks[idx] = r;
    }
    ranks
}

pub fn sorted_indices(results: &[Solution], opts: &SortOptions) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..results.len()).collect();
    if results.is_empty() || !opts.any() {
        return idx;
    }

    let mut total = vec![0usize; results.len()];
    let add = |values: Vec<f64>, reverse: bool, total: &mut Vec<usize>| {
        for (t, r) in total.iter_mut().zip(rank(&values, reverse)) {
            *t += r;
        }
    };

    if opts.by_dispensers {
        add(
            results.iter().map(|r| r.total_dispensers as f64).collect(),
            false,
            &mut total,
        );
    }
    if opts.by_gap {
        add(
            results.iter().map(|r| r.gap_avg).collect(),
            false,
            &mut total,
        );
    }
    if opts.by_sand_hammer_gap {
        add(
            results.iter().map(|r| r.sand_hammer_gap as f64).collect(),
            false,
            &mut total,
        );
    }
    if opts.by_distance {
        add(
            results.iter().map(|r| r.sand_distance).collect(),
            opts.distance_desc,
            &mut total,
        );
    }

    idx.sort_by_key(|&i| total[i]);
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solver_matches_python() {
        #[derive(Deserialize)]
        struct Case {
            name: String,
            params: Params,
            expected: Vec<Solution>,
        }
        #[derive(Deserialize)]
        struct Fixture {
            cases: Vec<Case>,
        }

        let fixture: Fixture =
            serde_json::from_str(include_str!("../tests/solver_fixture.json")).unwrap();

        for case in &fixture.cases {
            let got = solve(&case.params, &Arc::new(Progress::default()));
            assert_eq!(
                got.len(),
                case.expected.len(),
                "{} : nombre de solutions",
                case.name
            );
            for (i, (a, b)) in got.iter().zip(&case.expected).enumerate() {
                let fields: [(&str, f64, f64); 16] = [
                    ("sand_power", a.sand_power as f64, b.sand_power as f64),
                    ("sand_position", a.sand_position, b.sand_position),
                    ("sand_distance", a.sand_distance, b.sand_distance),
                    (
                        "sand_gametick",
                        a.sand_gametick as f64,
                        b.sand_gametick as f64,
                    ),
                    ("sand_velocity", a.sand_velocity, b.sand_velocity),
                    ("sand_position_y", a.sand_position_y, b.sand_position_y),
                    ("hammer_power", a.hammer_power as f64, b.hammer_power as f64),
                    ("hammer_position", a.hammer_position, b.hammer_position),
                    (
                        "hammer_gametick",
                        a.hammer_gametick as f64,
                        b.hammer_gametick as f64,
                    ),
                    (
                        "hammer_position_y",
                        a.hammer_position_y,
                        b.hammer_position_y,
                    ),
                    ("ratio_hammer", a.ratio_hammer as f64, b.ratio_hammer as f64),
                    ("ratio_velocity", a.ratio_velocity, b.ratio_velocity),
                    ("final_velocity_y", a.final_velocity_y, b.final_velocity_y),
                    (
                        "total_dispensers",
                        a.total_dispensers as f64,
                        b.total_dispensers as f64,
                    ),
                    ("gap_avg", a.gap_avg, b.gap_avg),
                    (
                        "sand_hammer_gap",
                        a.sand_hammer_gap as f64,
                        b.sand_hammer_gap as f64,
                    ),
                ];
                for (name, got_v, want_v) in fields {
                    assert_eq!(
                        got_v.to_bits(),
                        want_v.to_bits(),
                        "{} ligne {i} champ {name} : {got_v} != {want_v}",
                        case.name
                    );
                }
            }
        }
    }

    #[test]
    fn sort_matches_python() {
        #[derive(Deserialize)]
        struct Case {
            opts: SortOptions,
            order: Vec<usize>,
        }
        #[derive(Deserialize)]
        struct Fixture {
            rows: Vec<Solution>,
            cases: Vec<Case>,
        }

        let fixture: Fixture =
            serde_json::from_str(include_str!("../tests/sort_fixture.json")).unwrap();

        for (n, case) in fixture.cases.iter().enumerate() {
            let got = sorted_indices(&fixture.rows, &case.opts);
            assert_eq!(got, case.order, "combinaison #{n} : {:?}", case.opts);
        }
    }
}
