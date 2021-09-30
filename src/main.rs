//use gpio_cdev::{Chip, LineRequestFlags};
use stepper::StepperMotorApparatus;
use std::sync::{Arc, atomic::{AtomicI8, Ordering}};
use std::thread;


#[tokio::main]
async fn main() {
   let mut stepper = StepperMotorApparatus::new("/dev/gpiochip1","/dev/gpiochip3")
       .expect("StepperMotorApparatus Failed");

   let motor_state = Arc::new(AtomicI8::new(0));

   let mut arc_motor = Arc::new(stepper.stepper_motor);

   loop {
      let mut arc_motor = Arc::clone(&arc_motor);
      motor_state.fetch_add(1, Ordering::Relaxed);
      arc_motor.set_state(motor_state.clone())
          .await.expect("Set state for motor failed");
      thread::sleep_ms(2000);
      motor_state.fetch_sub(1, Ordering::Relaxed);
      arc_motor.set_state(motor_state.clone())
          .await.expect("Set state for motor failed");
      thread::sleep_ms(2000);
   }

}
