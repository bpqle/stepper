//use gpio_cdev::{Chip, LineRequestFlags};
use stepper::{State, StepperMotorApparatus};
use std::sync::{Arc, Mutex, };//atomic::{AtomicI8, Ordering}};
use std::time::Duration;
use std::thread;


#[tokio::main]
async fn main() {
   let mut stepper = StepperMotorApparatus::new("/dev/gpiochip1", "/dev/gpiochip3")
       .expect("StepperMotorApparatus Failed");

   //let mut motor_state = Arc::new(Mutex::new(State::Forward));

   loop {
      println!("starting new loop");
      &stepper.stepper_motor.set_state(State::Forward);
      thread::sleep(Duration::from_secs(2));
      &stepper.stepper_motor.set_state(State::Stop);
      thread::sleep(Duration::from_secs(2));
   }

}
