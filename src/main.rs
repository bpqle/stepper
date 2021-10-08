//use gpio_cdev::{Chip, LineRequestFlags};
use stepper::{State, StepperMotorApparatus};
use std::sync::{Arc, Mutex, };//atomic::{AtomicI8, Ordering}};
use std::time::Duration;
use std::thread;
use gpio_cdev::{Chip,
                LineRequestFlags, LineEventHandle,
                MultiLineHandle,
                EventRequestFlags, EventType,
                errors::Error as GpioError
};


#[tokio::main]
async fn main() {
   let mut stepper = StepperMotorApparatus::new("/dev/gpiochip1", "/dev/gpiochip3").await
       .expect("StepperMotorApparatus Failed");


   loop {
      println!("starting new loop");
      stepper.stepper_motor.set_state(State::Forward);
       thread::sleep(Duration::from_secs(1));
       println!("State is {:?}", &stepper.stepper_motor.state_txt.lock().unwrap());
      thread::sleep(Duration::from_secs(1));
      stepper.stepper_motor.set_state(State::Stop);
       thread::sleep(Duration::from_secs(1));
       println!("State is {:?}", &stepper.stepper_motor.state_txt.lock().unwrap());
      thread::sleep(Duration::from_secs(1));
   }

}
