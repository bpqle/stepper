use gpio_cdev::{Chip,
                LineRequestFlags, LineEventHandle,
                MultiLineHandle,
                EventRequestFlags, EventType,
                errors::Error as GpioError};
use tokio::{task::JoinHandle,
            sync::{oneshot, mpsc}};
use thiserror;
use std::{thread,
          time,
          sync::{Arc,
                 atomic::{AtomicI8, Ordering}}};
use futures::{pin_mut, TryFutureExt};
//use std::os::unix::io::AsRawFd;
//use nix::poll::PollFd;
//use structopt::StructOpt;
//use quicli::prelude::*;


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
    pub async fn set_state(self: &mut Arc<Self>, state_atm: Arc<AtomicI8>) -> Result<JoinHandle<()>, Error> {

        let self_clone = self.clone();
        let mut state_clone = state_atm.clone();

        let set_state_handle = tokio::spawn(async move {
            //let dt = 1000000 / 500;
            let num_half_steps: usize = 8;
            let mut step_values1 = &Self::ALL_OFF;
            let mut step_values3 = &Self::ALL_OFF;
            let mut step: usize = 0;

            loop {
                self_clone.motor_1_handle.set_values(&step_values1.0)
                    .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR1_OFFSETS });
                self_clone.motor_3_handle.set_values(&step_values3.0)
                    .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR3_OFFSETS });

                match state_clone.fetch_add(0, Ordering::Relaxed) {
                    1 => {
                        step = (step + 1) % &num_half_steps;
                        step_values1 = &Self::HALF_STEPS[step].0;
                        step_values3 = &Self::HALF_STEPS[step].1;
                    }
                    -1 => {
                        step = (step - 1) % &num_half_steps;
                        step_values1 = &Self::HALF_STEPS[step].0;
                        step_values3 = &Self::HALF_STEPS[step].1;
                    }
                    0 => {
                        step_values1 = &Self::ALL_OFF;
                        step_values3 = &Self::ALL_OFF;
                    }
                    _ => ()
                }
            }
        });
        Ok(set_state_handle)
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
