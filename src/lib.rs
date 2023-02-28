use gpio_cdev::{Chip,
                LineRequestFlags,
                AsyncLineEventHandle,
                EventRequestFlags, EventType,
                errors::Error as GpioError};
use tokio::{time::Duration,
};
use thiserror;
use std::{thread,
          sync::{Arc,
                 atomic::{AtomicU8, Ordering
                 }}
};
use futures::{pin_mut, Stream, StreamExt};
use log::{info, trace, warn};
//use std::fs::File;

struct LinesValue([u8; 2]);
pub struct StepperMotorApparatus {
    pub stepper_motor: StepperMotor,
    pub switch: Switch,
}
pub struct StepperMotor {
    pub state: Arc<AtomicU8>
}
pub struct Switch {
    switch_line_14: AsyncLineEventHandle,
    switch_line_15: AsyncLineEventHandle
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

    fn new(chip1: &mut Chip, chip3: &mut Chip) -> Result<Self, Error> {

        let state = Arc::new(AtomicU8::new(0));
        let state_clone = Arc::clone(&state);

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

        let _motor_thread  = thread::spawn(move || {
            let mut step: usize = 0;
            motor_1_handle.set_values(&Self::ALL_OFF.0)
                .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR1_OFFSETS })
                .unwrap();
            motor_3_handle.set_values(&Self::ALL_OFF.0)
                .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR3_OFFSETS })
                .unwrap();
            loop {
                match state_clone.load(Ordering::Relaxed) {
                    1 => {
                        step = (step + 1) % Self::NUM_HALF_STEPS;
                        let step_1_values = &Self::HALF_STEPS[step].0;
                        let step_3_values = &Self::HALF_STEPS[step].1;
                        motor_1_handle.set_values(&step_1_values.0)
                            .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR1_OFFSETS })
                            .unwrap();
                        motor_3_handle.set_values(&step_3_values.0)
                            .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR3_OFFSETS })
                            .unwrap();
                    },
                    0 => {
                        let step_1_values = &Self::ALL_OFF;
                        let step_3_values = &Self::ALL_OFF;
                        motor_1_handle.set_values(&step_1_values.0)
                            .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR1_OFFSETS })
                            .unwrap();
                        motor_3_handle.set_values(&step_3_values.0)
                            .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR3_OFFSETS })
                            .unwrap();
                    },
                    2 => {
                        step = (step - 1) % Self::NUM_HALF_STEPS;
                        let step_1_values = &Self::HALF_STEPS[step].0;
                        let step_3_values = &Self::HALF_STEPS[step].1;
                        motor_1_handle.set_values(&step_1_values.0).map_err(|e: GpioError|
                            Error::LinesSetError { source: e, lines: &Self::MOTOR1_OFFSETS }).unwrap();
                        motor_3_handle.set_values(&step_3_values.0).map_err(|e: GpioError|
                            Error::LinesSetError { source: e, lines: &Self::MOTOR3_OFFSETS }).unwrap();
                    },
                    _ => {warn!("Invalid state read"); continue}
                };
                thread::sleep(Duration::from_micros(Self::DT));
            }
        });

        Ok(StepperMotor {
            state
        })
    }
    pub fn set_state(&mut self, state: State) -> Result<(), Error> {
        match state {
            State::Forward => {self.state.store(1, Ordering::Relaxed);}
            State::Backward => {self.state.store(2, Ordering::Relaxed);}
            State::Stop => {self.state.store(0, Ordering::Relaxed);}
        }
        Ok(())
    }
}
impl Switch {
    //const SWITCH_OFFSETS: [u32;2] = [14,15];
    pub fn new(chip1: &mut Chip) -> Result<Self, Error> {

        let line_14 = chip1.get_line(14)
            .map_err(|e:GpioError| Error::LineGetError {source:e, line: 14}).unwrap();
        let handle_14 = AsyncLineEventHandle::new(line_14.events(
            LineRequestFlags::INPUT,
            EventRequestFlags::BOTH_EDGES,
            "stepper_motor_switch"
        ).map_err(|e: GpioError| Error::LineReqEvtError {source:e, line: 14}).unwrap())
            .map_err(|e: GpioError| Error::AsyncLineReqError {source: e, line: 14}).unwrap();

        let line_15 = chip1.get_line(15)
            .map_err(|e:GpioError| Error::LineGetError {source:e, line: 15}).unwrap();
        let handle_15 = AsyncLineEventHandle::new(line_15.events(
            LineRequestFlags::INPUT,
            EventRequestFlags::BOTH_EDGES,
            "stepper_motor_switch"
        ).map_err(|e: GpioError| Error::LineReqEvtError {source:e, line: 15}).unwrap())
            .map_err(|e: GpioError| Error::AsyncLineReqError {source: e, line: 15}).unwrap();

        Ok(Self{
            switch_line_14: handle_14,
            switch_line_15: handle_15
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
    pub async fn switch_ctrl(mut self) -> Result<(), Error> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = &mut self.switch.switch_line_14.next() => {

                        match event.unwrap().unwrap().event_type() {
                            EventType::RisingEdge => {
                                println!("Switch 14 de-pressed");
                                &self.stepper_motor.set_state(State::Stop);}
                            EventType::FallingEdge => {
                                println!("Switch 14 pressed");
                                &self.stepper_motor.set_state(State::Forward);}
                        }
                    }
                    event = &mut self.switch.switch_line_15.next() => {
                        match event.unwrap().unwrap().event_type() {
                            EventType::RisingEdge => {
                                println!("Switch 15 de-pressed");
                                &self.stepper_motor.set_state(State::Stop);}
                            EventType::FallingEdge => {
                                println!("Switch 15 pressed");
                                &self.stepper_motor.set_state(State::Backward);}
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
