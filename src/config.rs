//! The idea here is to have a minimal config file, that specifies only enough
//! parameters to uniquely define the forward model. Then, that `Config` object
//! can be parsed to a `System` object. In that process, the system is
//! initialised and any one-off initialisation tasks are performed.
//!
//! One slightly frustrating detail is this: In order for the config file to
//! be clear and human readable, a lot of the class names will be duplicated
//! with names from the main crate library. This is resolved by specifying,
//! e.g., `crate::Disturbance` for the normal non-config type, and simply
//! (e.g.) `Disturbance` within this config module.

use serde::{Deserialize, Serialize};
use std::{fs, rc::Rc};
use thiserror::Error;

use crate::System;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("toml serialization failed")]
    Serialization(#[from] toml::ser::Error),
    #[error("toml deserialization failed")]
    Deserialize(#[from] toml::de::Error),
    #[error("io error: {0}")]
    OpenConfig(#[from] std::io::Error),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    disturbances: Vec<Disturbance>,
    sensors: Vec<Sensor>,
    outputs: Vec<Output>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Source {
    /// id must be unique per config file
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
enum Disturbance {
    Zernike {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
enum Sensor {
    SHWFS {
        /// id must be unique per config file
        id: String,
        /// nsubs across aperture
        nsubx: usize,
        subwidth: f64,
        centre: (f64, f64),
        rotation: f64,
        direction: (f64, f64),
        gsalt: f64,
    },
    Imager {
        /// id must be unique per config file
        id: String,
        /// nsubs across aperture
        nsamples: usize,
        pitch: f64,
        centre: (f64, f64),
        rotation: f64,
        direction: (f64, f64),
        gsalt: f64,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Output {
    /// disturbance ids
    pub disturbances: Vec<String>,
    /// sensor ids
    pub sensors: Vec<String>,
    /// quality metric
    pub metric: Metric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Metric {
    WafefrontError,
}

impl<'a> Config {
    pub fn new() -> Self {
        Self {
            disturbances: vec![Disturbance::Zernike {
                id: "dm".to_string(),
                coeffs: vec![1.0, 2.0, 3.0],
                radius: 4.0,
                altitude: 0.0,
            }],
            sensors: vec![Sensor::SHWFS {
                id: "lgs1".to_string(),
                nsubx: 4,
                subwidth: 0.2,
                centre: (0.0, 0.0),
                rotation: 0.0,
                direction: (17.5, 0.0),
                gsalt: 90.0e3,
            }],
            outputs: vec![Output {
                disturbances: vec!["dm".to_string()],
                sensors: vec!["lgs1".to_string()],
                metric: Metric::WafefrontError,
            }],
        }
    }

    pub fn to_string(&self) -> Result<String, ConfigError> {
        let result = toml::to_string(self)?;
        Ok(result)
    }

    pub fn from_str(input: &str) -> Result<Self, ConfigError> {
        let config = toml::from_str(input)?;
        Ok(config)
    }

    pub fn from_file(filename: &str) -> Result<Self, ConfigError> {
        let string = fs::read_to_string(filename)?;
        let config = Self::from_str(&string)?;
        Ok(config)
    }

    pub fn to_file(&self, filename: &str) -> Result<(), ConfigError> {
        fs::write(filename, self.to_string()?)?;
        Ok(())
    }

    pub fn to_system(self) -> System {
        let sys_disturbances: Vec<Rc<crate::Disturbance>> = self
            .disturbances
            .into_iter()
            .map(
                |Disturbance::Zernike {
                     id,
                     coeffs,
                     radius,
                     altitude,
                 }| {
                    Rc::new(crate::Disturbance::new_zernike(
                        id, coeffs, radius, altitude,
                    ))
                },
            )
            .collect();
        let sys_sensors: Vec<Rc<crate::Sensor>> = self
            .sensors
            .into_iter()
            .map(|sensor| match sensor {
                Sensor::SHWFS {
                    id,
                    nsubx,
                    subwidth,
                    centre,
                    rotation,
                    direction,
                    gsalt,
                } => Rc::new(crate::Sensor::new_shwfs(
                    &id, nsubx, subwidth, centre, rotation, direction, gsalt,
                )),
                Sensor::Imager {
                    id,
                    nsamples,
                    pitch,
                    centre,
                    rotation,
                    direction,
                    gsalt,
                } => Rc::new(crate::Sensor::new_imager(&id, nsamples, pitch, centre, rotation, direction, gsalt)),
            })
            .collect();
        let sys_outputs: Vec<crate::Output> = self
            .outputs
            .into_iter()
            .map(
                |Output {
                     disturbances,
                     sensors,
                     metric,
                 }| crate::Output {
                    sensors: sys_sensors
                        .iter()
                        .filter_map(|p| {
                            match sensors.contains(match &**p {
                                crate::Sensor::SHWFS { id, .. } => id,
                                crate::Sensor::Imager { id, .. } => id,
                            }) {
                                true => Some(p.clone()),
                                false => None,
                            }
                        })
                        .collect(),
                    disturbances: sys_disturbances
                        .iter()
                        .filter_map(|p| {
                            match disturbances.contains(match &**p {
                                crate::Disturbance::Zernike { id, .. } => id,
                            }) {
                                true => Some(p.clone()),
                                false => None,
                            }
                        })
                        .collect(),
                    metric: match metric {
                        Metric::WafefrontError => crate::Metric::WavefrontError,
                    },
                },
            )
            .collect();
        System {
            outputs: sys_outputs,
        }
    }
}

impl Disturbance {}
