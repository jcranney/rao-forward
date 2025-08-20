## NCPA Calibration Technique Validations

The NCPA identification strategy for MAVIS is complex. It is summarised as:
 - perform phase diversity for a collection of bright diffraction limited sources over the science field,
 - reconstruct measured NCPAs tomographically,
 - project reconstructed NCPAs onto DMs in a way that minimises the residual NCPA over the science field,
 - apply those DM commands to the DM, and determine the measured WFS measurements that would be seen when those commands are applied.

This is complicated by the fact that the ground layer DM (the UT4 DSM) is not illuminated by the calibration source during NCPA calibrations - so the 3rd and 4th steps of this process require an accurate model of the DSM/WFS/Science interactions.

We aim here to simulate this entire process to a satisfactory level of realism, in order to learn more about the process in MAVIS and to grasp the expected levels of residual NCPAs.

### Plan
 - build a forward model, including:
    - DSM,
    - VDM,
    - DMHI,
    - DMLO,
    - LGS WFSs,
    - NGS WFSs,
    - Science imager,
    - Cal unit calibration sources,
    - VTB verification sources,
    - Arbitrary aberration sources, configurable to be visible by any combination of sensors/illuminated by any combination of sources.
    - WFS/imager noise,
    - source brightness,

Let's break that down:
 - Light Sources
    - NCPA Mask sources,
    - NGS Mask sources,
    - LGS Mask sources,
    - VTB Visible sources,
    - On-sky visible sources,
    - On-sky LGS sources,
    - On-sky NGS sources,
 - Wavefront disturbances
    - DSM,
    - VDM,
    - DMHI,
    - DMLO,
    - Various optical components in various optical paths,
 - Sensors (all noisy)
    - LGS WFSs,
    - NGS WFSs,
    - Science imager,

It feels like a yaml-style configuration here could be good. But not exactly sure of the best way to structure it. Let's assume we want to be able to run a simulation from the command line like:
```bash
simu-forward --config=my_experiment.yaml
```
which ought to run, and then output the resulting metrics (which should also be defined in the config file).

It seems that there is no need to include temporal dynamics in this, e.g., closing the loop etc. It should be very fast at running a single simulation, because I'd like to be able to run enough config files with slightly different configurations to generate a distribution of results.

It can probably assume geometric propagation, but it might be nice to structure it in a way that allows for Fresnel propagation in the future.

So rust? Could leverage a bit of RAO potentially, but doesn't seem like it needs to be part of the same package.

## Requirements
### Config file
 - the config file shall be in yaml format.
 - the config file shall allow for setting of:
   - Light source(s),
   - Wavefront disturbance(s),
   - Sensor(s),
   - Output metric(s).
### Performance
 - the simulations shall be very fast (<1 second, goal <0.1 second) to run a typical single forward model from the command line.
### Execution
 - the simulations shall be runnable from the command line by:
 ```bash
simu-forward --config=./my_experiment.yaml
# or equivalently:
simu-forward my_experiment.yaml
 ```
### Features
 - the simulations shall include geometric propagation (no scintillation) but allow for an extension to include geometric propagation in future developments.# rao-forward
