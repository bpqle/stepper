use gpio_cdev::{Chip,
                LineRequestFlags, LineEventHandle,
                LineHandle,
                AsyncLineEventHandle,
                MultiLineHandle,
                EventRequestFlags, EventType,
                errors::Error as GpioError};
use tokio::{task::JoinHandle,
            time::Duration,
};
use thiserror;
use std::{//thread,
          sync::{Arc, Mutex,
                 atomic::{AtomicI8, AtomicUsize, Ordering}}
};
use futures::{pin_mut, TryFutureExt, Stream, StreamExt};
use std::fs::File;

struct LinesValue([u8; 2]);
pub struct StepperMotorApparatus {
    pub stepper_motor: StepperMotor,
    pub switch: Switch,
}
pub struct StepperMotor {
    motor_1: MultiLineHandle,
    motor_3: MultiLineHandle,
    pub state: Arc<Mutex<State>>,
    pub state_txt: Arc<Mutex<String>>,
}
pub struct Switch {
    evt_handles: Vec<AsyncLineEventHandle>,
}
impl StepperMotor {
    const MOTOR1_OFFSETS: [u32;2] = [13,12];
    const MOTOR3_OFFSETS: [u32;2] = [19,21];
    const ALL_OFF: LinesValue = LinesValue([0,0]);
    const NUM_HALF_STEPS: usize = 8;
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

    async fn new(chip1: &mut Chip, chip3: &mut Chip) -> Result<Self, Error> {
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
        let state: Arc<Mutex<State>> = Arc::new(Mutex::new(State::default()));
        let state_txt = Arc::new(Mutex::new(String::from("Stop")));

        Ok(StepperMotor {
            motor_1: motor_1_handle,
            motor_3: motor_3_handle,
            state,
            state_txt,
        })
    }
    fn run_motor(&mut self) -> Result<(), Error> {

    }
    pub fn set_state(&mut self, new_state: State) -> Result<(), Error> {
        let mut motor_state = Arc::clone(&self.state);
        let mut motor_state = motor_state.lock().unwrap();

        match new_state {
            State::Forward => {
                //*state_txt = String::from("Forward");
                *motor_state = State::Forward;
            }
            State::Backward => {
                //*state_txt = String::from("Backward");
                *motor_state = State::Backward;
            }
            State::Stop => {
                //*state_txt = String::from("Stop");
                *motor_state = State::Stop;
            }
        }
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
    pub async fn new(chip1 : &str, chip3 : &str) -> Result<Self, Error> {
        let mut chip1 = Chip::new(chip1).map_err( |e:GpioError|
            Error::ChipError {source: e,
                chip: ChipNumber::Chip1}
        )?;
        let mut chip3 = Chip::new(chip3).map_err( |e:GpioError|
            Error::ChipError {source: e,
                chip: ChipNumber::Chip3}
        )?;
        let stepper_motor = StepperMotor::new(&mut chip1, &mut chip3).await?;
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
                        match event.unwrap().event_type() {
                            EventType::RisingEdge => self.stepper_motor.set_state(State::Forward).unwrap(),
                            EventType::FallingEdge => self.stepper_motor.set_state(State::Stop).unwrap()
                        }
                    }
                    event = self.switch.evt_handles[1].next() => {
                        match event.unwrap().event_type() {
                            EventType::RisingEdge => self.stepper_motor.set_state(State::Backward).unwrap(),
                            EventType::FallingEdge => self.stepper_motor.set_state(State::Stop).unwrap()
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
