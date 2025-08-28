## NCPA Calibration Technique Validations

The NCPA identification strategy for MAVIS is complex. It is summarised as:
 - perform phase diversity for a collection of bright diffraction limited sources over the science field,
 - reconstruct measured NCPAs tomographically,
 - project reconstructed NCPAs onto DMs in a way that minimises the residual NCPA over the science field,
 - apply those DM commands to the DM, and determine the measured WFS measurements that would be seen when those commands are applied.

This is complicated by the fact that the ground layer DM (the UT4 DSM) is not illuminated by the calibration source during NCPA calibrations - so the 3rd and 4th steps of this process require an accurate model of the DSM/WFS/Science interactions.

We aim here to simulate this entire process to a satisfactory level of realism, in order to learn more about the process in MAVIS and to grasp the expected levels of residual NCPAs.

## Requirements
### Config file
 - [x] the config file shall be in json format.
 - [x] the config file shall allow for setting of:
   - ~~Light source(s)~~,
   - [x] Wavefront disturbance(s),
   - [x] Sensor(s),
   - [x] Output metric(s).
### Performance
 - [ ] the simulations shall be very fast (<1 second, goal <0.1 second) to run a typical single forward model from the command line.
### Execution
 - [x] the simulations shall be runnable from the command line by:
 ```bash
rao-forward --input ./my_experiment.json
# or equivalently:
rao-forward -i ./my_experiment.json
# or using piped stdin:
cat my_experiment.json | rao-forward
 ```
 - [x] the outputs of the simulation shall be written to:
   - [x] stdout in json format,
   - [x] optional output file specified with `-o` flag.
### Features
 - [ ] the simulations shall include geometric propagation (no scintillation) but allow for an extension to include geometric propagation in future developments.
 ## Enums
 - [x] the tool shall support Disturbances of types:
   - [x] Zenike,
 - [ ] the tool shall support Sensors of types:
   - [x] SHWFS,
   - [ ] Phase,
   - [ ] Zernike Projection (? maybe this can be done outside the tool more efficiently),
 - [x] the tool shall support Outputs of types:
   - [x] RMS residual (scalar, in sensor units)
   - [x] measurement vector (vector, in sensor units)

## Plans (latest first)
### 28 Aug 2025
 - We now have a tool that allows the evaluation of image metrics for a specified optical system, defined by a `config.json` file.
 - Since the previous plan, most things went straightforward, except for the config file format. I started with YAML, but serde_yaml is deprecated and the alternatives don't grab my attention. I switched to toml for a while but I find the table declerations to be too wordy and a bit counterintuitive. I settled on json (for now), which is not as clean as yaml but at least it is intuitive and well supported. I've updated the requirements accordinginly.
 - Now how to use it to actually simulate the MAVIS NCPA calibration technique?

This seems like about the right time to define a test case, with some expected "result" and then build up the simulation tool from there. So let's consider a "minimal" MAVIS-like NCPA setup:
 - 3 LGS,
 - 3 NGS,
 - 4 NCPA calibration sources,
 - 4 on-sky sources (randomish positions),
 - 1 Ground layer DM (not seen during calibrations),
 - 1 High altitude DM (seen during calibratiosn),
 - 1 common path disturbance,
 - 1 lgs wfs path disturbance,
 - 1 science path disturbance,

Say we can "simulate" that, whatever that means. What is the expected way to use the result of the simulations? If the goal is to simulate the NCPA calibration method, then how close can we get to that using this tool. In summary, the NCPA calibration method is:
 1. close MCAO loop with available DMs to reduce common-path aberrations,
 2. freeze DMs in that state, and open MCAO loop,
 3. perform tomographic phase diversity at the imager to determine the optimal DM shapes (including external DMs) to maximise imager image-quality,
 4. determine corresponding WFS slopes that would be seen when those optimal DM shapes are taken,
 5. save the corresponding DM shapes and WFS slopes as the "references" in the MCAO loop,
 6. verify on-sky (or with the VTB) that the WFE is minimised when those references are applied.

Let's see what extra features this tool will need in order to perform each of those tasks:
 1. close loop
    - [TOOL] shall output measurements for use by a higher layer,
    - [SOLVED-TOOL] shall allow for commands to be sent (e.g., by modifying the values of the DM zernike coefficients),
    - [HIGHER_LAYER] shall perform interaction matrix/control matrix evaluation, and loop control management. Effectively the higher layer needs to be a fully fledged RTC. I think I know a guy.
 2. set DM shape
    - [DUPLICATE-TOOL] allow for commands to be sent,
 3. tomographic phase diversity (simulate)
    - [TOOL] measure residual modes in set of directions,
    - [HIGHER_LAYER] tomographically fit those modes to the available DMs,
    - [DUPLICATE-TOOL] allow for commands to be sent,
 4. estimate slope reference
    - [HIGHER_LAYER] estimate WFS slopes that would be seen for those commands (e.g., using previously acquired interaction matrix).
 5. save references in MCAO loop
    - [HIGHER_LAYER] save references in MCAO loop,
 6. verification
    - [SOLVED-TOOL] measure residual wfe using on-sky or VTB sources.

Extracting the unique requirements from this, there are those for the tool itself:
 - shall output measurements for use by a higher layer,
 - measure residual modes in set of directions,

These can be mapped to simple extensions of the existing tool:
 - add "measurement vector" output type (e.g., slope vector for SHWFS, phase for phase sensor)
 - add "zernike projection" output type. This may be heavy.

It occurs to me that the higher layer tool might benefit from being able to pipe in configuration data through `stdin`, since it will likely be reading the results through `stdout`. So let's add that requirement too.

### 5 Aug 2025
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
