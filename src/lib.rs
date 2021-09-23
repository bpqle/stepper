use gpio_cdev::{Chip, LineRequestFlags, LineEventHandle, EventRequestFlags, EventType, errors::Error as GpioError, MultiLineHandle};
use tokio::sync::mpsc;
use thiserror;
use std::{thread, time};
use std::os::unix::io::AsRawFd;
use nix::poll::PollFd;
use structopt::StructOpt;
use quicli::prelude::*;


struct LinesValue([u8; 2]);
pub struct StepperMotorApparatus {
    chip1: Chip,
    chip3: Chip,
    pub stepper_motor: StepperMotor,
    pub switch: Switch,
}
pub struct StepperMotor {
    motor_1_handle: MultiLineHandle,
    motor_3_handle: MultiLineHandle,
}
pub struct Switch {
    handle: MultiLineHandle,
}

impl StepperMotor {
     const MOTOR1_OFFSETS: [u32;2] = [13,12];
     const MOTOR3_OFFSETS: [u32;2] = [19,21];
     const ALL_OFF: LinesValue = LinesValue([0,0]);
     const HALF_STEPS: [(LinesValue, LinesValue); 8] = [
        (LinesValue([0,1]),LinesValue([1,0])),
        (LinesValue([0,1]),LinesValue([0,0])),
        (LinesValue([0,1]),LinesValue([0,1])),
        (LinesValue([0,0]),LinesValue([0,1])),
        (LinesValue([1,0]),LinesValue([0,1])),
        (LinesValue([1,0]),LinesValue([0,0])),
        (LinesValue([1,0]),LinesValue([1,0])),
        (LinesValue([0,0]),LinesValue([1,0]))
     ];

    pub fn new(chip1: &mut Chip, chip3: &mut Chip) -> Result<Self, Error> {
        let motor_1_handle = chip1
            .get_lines(&Self::MOTOR1_OFFSETS)
            .map_err(|e:GpioError| Error::LinesGetError {source: e, lines: &Self::MOTOR1_OFFSETS})?
            .request(LineRequestFlags::OUTPUT, &[0,0], "stepper")
            .map_err(|e:GpioError| Error::LinesReqError {source: e, lines: &Self::MOTOR1_OFFSETS})?;

        let motor_3_handle = chip3
            .get_lines(&Self::MOTOR3_OFFSETS)
            .map_err(|e:GpioError| Error::LinesGetError {source: e, lines: &Self::MOTOR3_OFFSETS})?
            .request(LineRequestFlags::OUTPUT, &[0,0], "stepper")
            .map_err(|e:GpioError| Error::LinesReqError {source: e, lines: &Self::MOTOR3_OFFSETS})?;

        Ok(StepperMotor {
            motor_1_handle,
            motor_3_handle,
        })
    }
    //TODO: resolve async type future()?
    pub fn set_state(&mut self, state: State) -> Result<(), Error> {
        //let dt = 1000000 / 500;

            let num_half_steps:usize = 8;
            let step_values1: &LinesValue;
            let step_values3: &LinesValue;
            let mut step: usize = 0;

            match state {
                State::Forward => {
                    step = (step + 1) % &num_half_steps;
                    step_values1 = &Self::HALF_STEPS[step].0;
                    step_values3 = &Self::HALF_STEPS[step].1;
                }
                State::Backward => {
                    step = (step - 1) % &num_half_steps;
                    step_values1 = &Self::HALF_STEPS[step].0;
                    step_values3 = &Self::HALF_STEPS[step].1;
                }
                State::Stop => {
                    step_values1 = &Self::ALL_OFF;
                    step_values3 = &Self::ALL_OFF;
                }
            };
            //TODO: add sleep
            Ok(
                &self.motor_1_handle.set_values(&step_values1.0)
                    .map_err(|e:GpioError| Error::LinesSetError {source: e, lines:&Self::MOTOR1_OFFSETS})
                &self.motor_3_handle.set_values(&step_values3.0)
                    .map_err(|e:GpioError| Error::LinesSetError {source: e, lines:&Self::MOTOR3_OFFSETS})
            )

    }
}

impl Switch {
    const SWITCH_OFFSETS: [u32;2] = [14,15];
    pub fn new(chip1: &mut Chip) -> Result<Self, Error> {
        let handle = chip1
            .get_lines(&Self::SWITCH_OFFSETS)
            .map_err(|e:GpioError| Error::LinesGetError {source: e, lines: &Self::SWITCH_OFFSETS})?
            .request(LineRequestFlags::INPUT, &[0,0], "stepper")
            .map_err(|e:GpioError| Error::LinesReqError {source: e, lines: &Self::SWITCH_OFFSETS})?;
        Ok(Self{
            handle
        })
    }
    pub async fn gpio_monitor(apparatus: &mut StepperMotorApparatus, num_events: u32) -> Result<State, Error> {
        let mut evt_handles: Vec<LineEventHandle> = Self::SWITCH_OFFSETS
            .into_iter()
            .map(|offset| {
                let line = apparatus.chip1.get_line(offset)
                    .map_err(|e:GpioError| Error::LinesGetError {source: e, lines: &Self::SWITCH_OFFSETS})?;
                line.events(
                    LineRequestFlags::INPUT,
                    EventRequestFlags::BOTH_EDGES,
                    "monitor",
                )
                    .map_err(|e:GpioError| Error::LinesReqError {source: e, lines: &Self::SWITCH_OFFSETS})?;
            })
            .collect();

        // Create a vector of file descriptors for polling
        let mut pollfds: Vec<PollFd> = evt_handles
            .into_iter()
            .map(|handle| {
                PollFd::new(
                    handle.as_raw_fd(),
                    PollEventFlags::POLLIN | PollEventFlags::POLLPRI,
                )
            })
            .collect();

        let (mon_transmit, mon_receive) = mpsc::channel(100);

        while let Some(evt) = mon_receive.recv().await {
            tokio::spawn()
        }

    }
}
impl StepperMotorApparatus {
    pub fn new(chip1 : &str, chip3 : &str) -> Result<Self, Error> {
        let mut chip1 = Chip::new(chip1).map_err( |e:GpioError|
            Error::ChipError {source: e,
                chip: ChipNumber::Chip1}
        )?;
        let mut chip3 = Chip::new(chip3).map_err( |e:GpioError|
            Error::ChipError {source: e,
                chip: ChipNumber::Chip3}
        )?;
        let stepper_motor = StepperMotor::new(&mut chip1, &mut chip3)?;
        let switch = Switch::new(&mut chip1)?;
        Ok(StepperMotorApparatus{
            chip1,
            chip3,
            stepper_motor,
            switch,
        })
    }
}

pub enum State{
    Forward,
    Backward,
    Stop,
}
#[derive(Debug)]
pub enum ChipNumber {
    Chip1,
    Chip3
}
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to get chip {chip:?}")]
    ChipError {
        source: GpioError,
        chip: ChipNumber,
    },
    #[error("Failed to get lines")]
    LinesGetError {
        source: GpioError,
        lines: &'static [u32; 2],
    },
    #[error("Failed to request lines")]
    LinesReqError {
        source: GpioError,
        lines: &'static [u32; 2],
    },
    #[error("Failed to set lines")]
    LinesSetError {
        source: GpioError,
        lines: &'static [u32; 2],
    },
    #[error("Failed to monitor switch lines")]
    SwitchMonitorError {
        source: GpioError,
        lines: &'static [u32; 2],
    }
}
