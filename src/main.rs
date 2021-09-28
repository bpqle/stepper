use gpio_cdev::{Chip, LineRequestFlags};
use stepper::StepperMotorApparatus;



fn main() {
   let mut stepper = StepperMotorApparatus::new("/dev/gpiochip1","/dev/gpiochip3")
       .expect("StepperMotorApparatus Failed");

   loop {

   }

}
