pub const AIR_DRAG: f64 = 0.980_000_019_073_486_3;
pub const GRAVITY: f64 = 0.04;
pub const HEAD_BLOCK: f64 = 0.019_999_980_926_514;

#[derive(Clone, Copy, Debug, Default)]
pub struct Barrel {
    pub eff_x: f64,
    pub eff_y: f64,
    pub eff_z: f64,
    pub dist_eff: f64,
}

impl Barrel {
    #[inline]
    pub fn eff_on(&self, axis: crate::solver::Axis) -> f64 {
        match axis {
            crate::solver::Axis::X => self.eff_x,
            crate::solver::Axis::Z => self.eff_z,
        }
    }
}

#[inline]
pub fn barrel_calc(
    power_x: f64,
    power_y: f64,
    power_z: f64,
    proj_x: f64,
    proj_y: f64,
    proj_z: f64,
) -> Barrel {
    let delta_x = proj_x - power_x;
    let delta_y = proj_y - power_y;
    let delta_z = proj_z - power_z;

    let distance = (delta_x * delta_x + delta_y * delta_y + delta_z * delta_z).sqrt();

    Barrel {
        eff_x: delta_x / distance,
        eff_y: delta_y / distance,
        eff_z: delta_z / distance,
        dist_eff: 1.0 - distance / 8.0,
    }
}
