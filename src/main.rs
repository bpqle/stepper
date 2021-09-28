//use gpio_cdev::{Chip, LineRequestFlags};
use stepper::StepperMotorApparatus;
use std::sync::{Arc, atomic::{AtomicI8, Ordering}};


#[tokio::main]
async fn main() {
   let mut stepper = StepperMotorApparatus::new("/dev/gpiochip1","/dev/gpiochip3")
       .expect("StepperMotorApparatus Failed");

   let motor_state = Arc::new(AtomicI8::new(1));
   Arc::new(stepper.stepper_motor).set_state(motor_state.clone())
       .await.expect("Set state for motor failed");
   motor_state.fetch_add(0, Ordering::Relaxed);
}
