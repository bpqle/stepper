use gpio_cdev::{Chip,
                LineRequestFlags, LineEventHandle,
                MultiLineHandle,
                EventRequestFlags, EventType,
                errors::Error as GpioError
};
use tokio::{task::JoinHandle,
            time::Duration,
            //sync::{oneshot, mpsc}
};
use thiserror;
use std::{//thread,

          sync::{Arc, Mutex,
                 atomic::{AtomicI8, AtomicUsize, Ordering}}
};
use futures::{pin_mut, TryFutureExt};
use std::fs::File;



struct LinesValue([u8; 2]);
pub struct StepperMotorApparatus {
    chip1: Chip,
    chip3: Chip,
    pub stepper_motor: StepperMotor,
    pub switch: Switch,
}
pub struct StepperMotor {
    //motor_1_handle: Arc<Mutex<MultiLineHandle>>,
    //motor_3_handle: Arc<Mutex<MultiLineHandle>>,
    pub state: Arc<Mutex<State>>,
    pub state_txt: Arc<Mutex<String>>,
}
pub struct Switch {
    handle: MultiLineHandle,
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

    pub async fn new(chip1: &mut Chip, chip3: &mut Chip) -> Result<Self, Error> {
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

        Self::run_motor(motor_1_handle, motor_3_handle, &state, &state_txt).await.unwrap();

        Ok(StepperMotor {
            state,
            state_txt
        })
    }

    async fn run_motor(m1_handle:MultiLineHandle, m3_handle: MultiLineHandle, state_arc: &Arc<Mutex<State>>, state_txt: &Arc<Mutex<String>>)
        -> Result<(), Error> {

        let state = Arc::clone(state_arc);
        let state_txt = Arc::clone(&state_txt);

        tokio::spawn(async move {

            let mut step_values1 = &Self::ALL_OFF;
            let mut step_values3 = &Self::ALL_OFF;
            let mut step: usize = 0;

            loop {
                let mut state = state.lock().unwrap();
                let mut state_txt = state_txt.lock().unwrap();

                match *state {
                    State::Forward => {
                        *state_txt = String::from("Forward1");
                        step = (step + 1) % &Self::NUM_HALF_STEPS;
                        step_values1 = &Self::HALF_STEPS[step].0;
                        step_values3 = &Self::HALF_STEPS[step].1;
                    }
                    State::Backward => {
                        *state_txt = String::from("Backward1");
                        step = (step - 1) % &Self::NUM_HALF_STEPS;
                        step_values1 = &Self::HALF_STEPS[step].0;
                        step_values3 = &Self::HALF_STEPS[step].1;
                    }
                    State::Stop => {
                        *state_txt = String::from("Stop1");
                        step_values1 = &Self::ALL_OFF;
                        step_values3 = &Self::ALL_OFF;
                    }
                };

                m1_handle.set_values(&step_values1.0)
                    .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR1_OFFSETS })
                    .unwrap();
                m3_handle.set_values(&step_values3.0)
                    .map_err(|e: GpioError| Error::LinesSetError { source: e, lines: &Self::MOTOR3_OFFSETS })
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(*&Self::DT));
                //let mut file = File::create("foo.txt").unwrap();
            };
        });
        Ok(())
    }

    pub fn set_state(&mut self, new_state: State) -> Result<(), Error> {
        let mut motor_state = Arc::clone(&self.state);
        let mut motor_state = motor_state.lock().unwrap();
        //let mut state_txt = Arc::clone(&self.state_txt);
        //let mut state_txt = state_txt.lock().unwrap();

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
