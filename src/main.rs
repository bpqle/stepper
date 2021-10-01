//use gpio_cdev::{Chip, LineRequestFlags};
use stepper::{State, StepperMotorApparatus};
use std::sync::{Arc, Mutex, };//atomic::{AtomicI8, Ordering}};
use std::time::Duration;
use std::thread;


#[tokio::main]
async fn main() {
   let mut stepper = StepperMotorApparatus::new("/dev/gpiochip1", "/dev/gpiochip3")
       .expect("StepperMotorApparatus Failed");

   let mut motor_state = Arc::new(Mutex::new(State::Forward));

   loop {
      println!("starting new loop");
      *motor_state.lock().unwrap() = State::Forward;
      &stepper.stepper_motor.set_state(&motor_state);
      thread::sleep(Duration::new(2,0));
      *motor_state.lock().unwrap() = State::Forward;
      &stepper.stepper_motor.set_state(&motor_state);
      thread::sleep(Duration::new(2,0));
   }

}
