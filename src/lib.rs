pub mod config;

use core::f64;
use std::sync::Arc;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use thiserror::Error;

pub use config::Config;
use rao::{Line, Measurement, Sampleable, Sampler, Vec2D, Vec3D};
use serde::{Deserialize, Serialize};

#[derive(Error, Debug)]
pub enum ResultsError {
    #[error("could not serialize results output")]
    Serialization(#[from] serde_json::Error),
}

const AS2RAD: f64 = f64::consts::PI / 180.0 / 3600.0;

pub struct System {
    pub outputs: Vec<Output>,
}

enum Disturbance {
    Zernike {
        /// indicies to interact with zernike module
        jnm: Vec<(u32, u32, u32)>,
        /// id must be unique per config file
        id: String,
        /// zernike coefficients
        coeffs: Vec<f64>,
        /// basis radius (in metres)
        radius: f64,
        /// altitude
        altitude: f64,
    },
}
enum Sensor {
    Shwfs {
        id: String,
        measurements: Vec<Measurement>,
    },
    Imager {
        id: String,
        measurements: Vec<Measurement>,
    },
}
pub struct Output {
    id: String,
    sensors: Vec<Arc<Sensor>>,
    disturbances: Vec<Arc<Disturbance>>,
    metric: Metric,
}

enum Metric {
    WavefrontError,
    MeasurementVector,
}

impl Disturbance {
    fn new_zernike(id: String, coeffs: Vec<f64>, radius: f64, altitude: f64) -> Self {
        // for n radial orders, there are:
        // l = n(n+1)/2 modes
        // so for l modes, you need at least
        // n = ((-1 + (1+8l).sqrt())/2).ceil
        let minimum_radial_orders: usize =
            ((-1.0 + (1.0 + 8.0 * coeffs.len() as f64)).sqrt() / 0.5).ceil() as usize;
        let mut jnm: Vec<(u32, u32, u32)> = vec![];
        let jnm_old = zernike::jnm(minimum_radial_orders as u32);
        for i in 0..(((minimum_radial_orders + 1) * minimum_radial_orders) / 2) {
            jnm.push((jnm_old.0[i], jnm_old.1[i], jnm_old.2[i]));
        }
        Disturbance::Zernike {
            id,
            coeffs,
            radius,
            altitude,
            jnm,
        }
    }
}

impl Sampleable for Disturbance {
    fn sample(&self, p: &rao::Line) -> f64 {
        match self {
            Self::Zernike {
                coeffs,
                jnm,
                radius,
                altitude,
                ..
            } => {
                let pos = p.position_at_altitude(*altitude);
                let r = pos.norm() / radius;
                let theta = pos.y.atan2(pos.x);
                (0..coeffs.len()).into_iter()
                    .map(|i| coeffs[i] * zernike::zernike(jnm[i].0, jnm[i].1, jnm[i].2, r, theta))
                    .sum()
            }
        }
    }
}

impl Sensor {
    fn new_shwfs(
        // id used for display purposes only here
        id: &str,
        // number of subapertures in each dimensions of MLA
        nsubx: usize,
        // subaperture width in metres (wrt pupil)
        subwidth: f64,
        // centre of MLA in metres (wrt pupil)
        centre: (f64, f64),
        // rotation of MLA wrt pupil in degrees
        rotation: f64,
        // direction of GS in arcsec
        direction: (f64, f64),
        // guide star altitude in metres
        gsalt: f64,
    ) -> Self {
        let x = rao::Vec2D::linspread(
            &Vec2D::new(-subwidth * (nsubx as f64) / 2.0, 0.0),
            &Vec2D::new(subwidth * (nsubx as f64) / 2.0, 0.0),
            nsubx as u32,
        );
        let y = rao::Vec2D::linspread(
            &Vec2D::new(0.0, -subwidth * (nsubx as f64) / 2.0),
            &Vec2D::new(0.0, subwidth * (nsubx as f64) / 2.0),
            nsubx as u32,
        );
        let centre = Vec2D::new(centre.0, centre.1);
        let mut centres = y
            .into_iter()
            .flat_map(move |y| {
                let c = centre.clone();
                x.clone().into_iter().map(move |x| x + &y + &c)
            })
            .collect::<Vec<Vec2D>>();
        // rotate the centres
        let rotation_rad = rotation * f64::consts::PI / 180.0;
        centres = centres
            .into_iter()
            .map(|c| {
                Vec2D::new(
                    c.x * rotation_rad.cos() - c.y * rotation_rad.sin(),
                    c.x * rotation_rad.sin() + c.y * rotation_rad.cos(),
                )
            })
            .collect();
        let axis = Line::new(0.0, direction.0 * AS2RAD, 0.0, direction.1 * AS2RAD);
        let gspos3d = Vec3D::new(
            axis.position_at_altitude(gsalt).x,
            axis.position_at_altitude(gsalt).y,
            gsalt,
        );
        let x_slopes: Vec<Measurement> = centres
            .iter()
            .map(|c| Measurement::SlopeTwoEdge {
                central_line: Line::new_from_two_points(&Vec3D::new(c.x, c.y, 0.0), &gspos3d),
                edge_length: subwidth,
                edge_separation: subwidth,
                gradient_axis: Vec2D::new(rotation_rad.cos(), rotation_rad.sin()),
                npoints: 2,
                altitude: f64::INFINITY,
            })
            .collect();
        let y_slopes: Vec<Measurement> = centres
            .iter()
            .map(|c| Measurement::SlopeTwoEdge {
                central_line: Line::new_from_two_points(&Vec3D::new(c.x, c.y, 0.0), &gspos3d),
                edge_length: subwidth,
                edge_separation: subwidth,
                gradient_axis: Vec2D::new(
                    (rotation_rad + f64::consts::FRAC_PI_2).cos(),
                    (rotation_rad + f64::consts::FRAC_PI_2).sin(),
                ),
                npoints: 2,
                altitude: f64::INFINITY,
            })
            .collect();
        let slopes: Vec<Measurement> = [x_slopes, y_slopes].concat();
        Self::Shwfs {
            id: id.to_string(),
            measurements: slopes,
        }
    }

    fn new_imager(
        // id used for display purposes only here
        id: &str,
        // number of samples in each dimensions of pupil
        nsample: usize,
        // distance between adjacent phase points in metres (wrt pupil)
        pitch: f64,
        // centre of MLA in metres (wrt pupil)
        centre: (f64, f64),
        // rotation of MLA wrt pupil in degrees
        rotation: f64,
        // direction of GS in arcsec
        direction: (f64, f64),
        // guide star altitude in metres
        gsalt: f64,
    ) -> Self {
        let x = rao::Vec2D::linspread(
            &Vec2D::new(-pitch * (nsample as f64) / 2.0, 0.0),
            &Vec2D::new(pitch * (nsample as f64) / 2.0, 0.0),
            nsample as u32,
        );
        let y = rao::Vec2D::linspread(
            &Vec2D::new(-pitch * (nsample as f64) / 2.0, 0.0),
            &Vec2D::new(pitch * (nsample as f64) / 2.0, 0.0),
            nsample as u32,
        );
        let centre = Vec2D::new(centre.0, centre.1);
        let mut centres = y
            .into_iter()
            .flat_map(move |y| {
                let c = centre.clone();
                x.clone().into_iter().map(move |x| x + &y + &c)
            })
            .collect::<Vec<Vec2D>>();
        let rotation_rad = rotation * f64::consts::PI / 180.0;
        centres = centres
            .into_iter()
            .map(|c| {
                Vec2D::new(
                    c.x * rotation_rad.cos() - c.y * rotation_rad.sin(),
                    c.x * rotation_rad.sin() + c.y * rotation_rad.cos(),
                )
            })
            .collect();
        let axis = Line::new(0.0, direction.0 * AS2RAD, 0.0, direction.1 * AS2RAD);
        let gspos3d = Vec3D::new(
            axis.position_at_altitude(gsalt).x,
            axis.position_at_altitude(gsalt).y,
            gsalt,
        );
        let meas: Vec<Measurement> = centres
            .iter()
            .map(|c| Measurement::Phase {
                line: Line::new_from_two_points(&Vec3D::new(c.x, c.y, 0.0), &gspos3d),
            })
            .collect();
        Self::Imager {
            id: id.to_string(),
            measurements: meas,
        }
    }
}

impl Metric {
    pub fn evaluate(&self, sensor: &Sensor, disturbances: Vec<Arc<Disturbance>>) -> Vec<f64> {
        match self {
            Metric::WavefrontError => match sensor {
                Sensor::Shwfs { measurements, .. } => {
                    let mut rms: f64 = 0.0;
                    for measurement in measurements {
                        let mut total_disturbance: f64 = 0.0;
                        for disturbance in disturbances.clone() {
                            total_disturbance += measurement.sample(disturbance.as_ref());
                        }
                        rms += total_disturbance.powf(2.0);
                    }
                    rms /= measurements.len() as f64;
                    vec![rms.sqrt()] // arcsec
                }
                Sensor::Imager { measurements, .. } => {
                    let mut rms: f64 = 0.0;
                    let mut mean: f64 = 0.0;
                    for measurement in measurements {
                        let mut total_disturbance: f64 = 0.0;
                        for disturbance in disturbances.clone() {
                            total_disturbance += measurement.sample(disturbance.as_ref());
                        }
                        rms += total_disturbance.powf(2.0);
                        mean += total_disturbance;
                    }
                    rms /= measurements.len() as f64;
                    mean /= measurements.len() as f64;
                    rms -= mean.powf(2.0);
                    rms = rms.sqrt();
                    vec![rms] // radians
                }
            },
            Metric::MeasurementVector => match sensor {
                Sensor::Shwfs { measurements, .. } => {
                    measurements
                        .par_iter()
                        .map(|meas| {
                            disturbances
                                .clone()
                                .into_par_iter()
                                .map(|dist| meas.sample(dist.as_ref()))
                                .sum()
                        })
                        .collect() // arcsec
                }
                Sensor::Imager { measurements, .. } => {
                    measurements
                        .par_iter()
                        .map(|meas| {
                            disturbances
                                .clone()
                                .into_iter()
                                .map(|dist| meas.sample(dist.as_ref()))
                                .sum()
                        })
                        .collect() // radians
                }
            },
        }
    }
}

impl Output {
    pub fn evaluate(&self) -> SimulationResult {
        let mut result = SimulationResult::new_from_output(self);
        let values: Vec<f64> = self.sensors
            .par_iter()
            .flat_map(|sensor| 
                self.metric.evaluate(sensor, self.disturbances.clone())
            ).collect();
        result.values = values;
        result
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimulationResult {
    pub id: String,
    pub values: Vec<f64>,
}

impl SimulationResult {
    fn new_from_output(output: &Output) -> Self {
        Self {
            id: output.id.clone(),
            values: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimulationResults {
    pub results: Vec<SimulationResult>,
}

impl Default for SimulationResults {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulationResults {
    pub fn new() -> Self {
        Self { results: vec![] }
    }

    pub fn to_string(&self) -> Result<String, ResultsError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

impl System {
    pub fn evaluate(&self) -> SimulationResults {
        SimulationResults {
            results: self
                .outputs
                .iter()
                .map(|output| output.evaluate())
                .collect(),
        }
    }
}
