use gpio_cdev::{Chip,
                LineRequestFlags, LineEventHandle,
                LineHandle,
                AsyncLineEventHandle,
                MultiLineHandle,
                EventRequestFlags, EventType,
                errors::Error as GpioError};
use tokio::{task::JoinHandle,
            time::Duration,
            sync::mpsc,
};
use thiserror;
use std::{num,
          sync::{Arc, Mutex,
                 atomic::{AtomicI8, AtomicUsize, Ordering}}
};
use futures::{pin_mut, TryFutureExt, Stream, StreamExt};
use std::fs::File;
use tokio::sync::mpsc::{Receiver, Sender};

struct LinesValue([u8; 2]);
pub struct StepperMotorApparatus {
    pub stepper_motor: StepperMotor,
    pub switch: Switch,
}
pub struct StepperMotor {
    motor_1: Arc<Mutex<MultiLineHandle>>,
    motor_3: Arc<Mutex<MultiLineHandle>>,
}
pub struct Switch {
    evt_handles: Vec<AsyncLineEventHandle>,
}
impl StepperMotor {
    const MOTOR1_OFFSETS: [u32;2] = [13,12];
    const MOTOR3_OFFSETS: [u32;2] = [19,21];
    const ALL_OFF: LinesValue = LinesValue([0,0]);
    const NUM_HALF_STEPS: isize = 8;
    const DT: u64 = 1000000/500;

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

    fn new(chip1: &mut Chip, chip3: &mut Chip) -> Result<Self, Error> {
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

        let motor_1_handle = Arc::new(Mutex::new(motor_1_handle));
        let motor_3_handle = Arc::new(Mutex::new(motor_3_handle));

        Ok(StepperMotor {
            motor_1: motor_1_handle,
            motor_3: motor_3_handle,
        })
    }
    async fn run_motor(&mut self, state: State) -> JoinHandle<()> {
        let motor_1 = Arc::clone(&self.motor_1);
        let motor_3 = Arc::clone(&self.motor_3);
        let mut step: isize = 0;
        let mut i: isize = 0;

        tokio::spawn(async move{
            let motor_1 = motor_1.lock().unwrap();
            let motor_3 = motor_3.lock().unwrap();
            loop {
                match state {
                    State::Forward => { i = 1;},
                    State::Backward => { i = -1;},
                    State::Stop => { i = 0;}
                }
                step = (step + i) % Self::NUM_HALF_STEPS;
                let ustep = usize::from(step);
                let step_1_values = &Self::HALF_STEPS[ustep].0;
                let step_3_values = &Self::HALF_STEPS[ustep].1;
                motor_1.set_values(&step_1_values.0)
                    .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR1_OFFSETS })
                    .unwrap();
                motor_3.set_values(&step_3_values.0)
                    .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR3_OFFSETS })
                    .unwrap();
                //TODO: sleep??
            }
        })
    }
    pub async fn set_state(&mut self, new_state: State) -> Result<(), Error> {
        Ok(())
    }
}
impl Switch {
    const SWITCH_OFFSETS: [u32;2] = [14,15];
    pub fn new(chip1: &mut Chip) -> Result<Self, Error> {
        let mut evt_handles: Vec<AsyncLineEventHandle> = (&Self::SWITCH_OFFSETS).iter()
            .map(|&offset|{
                let line = chip1.get_line(offset)
                    .map_err(|e:GpioError| Error::LineGetError {source:e, line: offset}).unwrap();
                let event = AsyncLineEventHandle::new(line.events(
                    LineRequestFlags::INPUT,
                    EventRequestFlags::BOTH_EDGES,
                    "stepper_motor_switch"
                ).map_err(|e: GpioError| Error::LineReqEvtError {source:e, line: offset}).unwrap())
                    .map_err(|e: GpioError| Error::AsyncLineReqError {source: e, line: offset}).unwrap();
                event
            }).collect();
        Ok(Self{
            evt_handles
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
            stepper_motor,
            switch,
        })
    }
    pub async fn switch_ctrl(self) -> Result<(), Error> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = self.switch.evt_handles[0].next() => {
                        match event.unwrap().unwrap().event_type() {
                            EventType::RisingEdge => {
                                tokio::select! {
                                    _ = &self.stepper_motor.set_state(State::Forward) => {println!("Switch not released, task expired??")}
                                    _ = self.switch.evt_handles[0].next() => {println!("Switch released!")}
                                }
                            }
                            _ => {println!("Problem")}
                        }
                    }
                    event = self.switch.evt_handles[1].next() => {
                        match event.unwrap().unwrap().event_type() {
                            EventType::RisingEdge => {
                                tokio::select! {
                                    _ = &self.stepper_motor.set_state(State::Backward) => {println!("Switch not released, task expired??")}
                                    _ = self.switch.evt_handles[1].next() => {println!("Switch released!")}
                                }
                            }
                            _ => {println!("Problem")}
                        }
                    }
                }
            }
        });
        Ok(())
    }
}

pub enum State{
    Forward,
    Backward,
    Stop,
}
impl Default for State{
    fn default() -> Self {
        State::Stop
    }
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
    #[error("Failed to get line")]
    LineGetError {
        source: GpioError,
        line: u32,
    },
    #[error("Failed to request line")]
    LineReqError {
        source: GpioError,
        line: u32,
    },
    #[error("Failed to request event handle for line")]
    LineReqEvtError {
        source: GpioError,
        line: u32,
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
    #[error("Failed to request async event handle")]
    AsyncLineReqError {
        source: GpioError,
        line: u32,
    },
    #[error("Failed to monitor switch lines")]
    SwitchMonitorError {
        source: GpioError,
        lines: &'static [u32; 2],
    }
}
