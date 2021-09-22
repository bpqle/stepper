use gpio_cdev::{Chip, LineRequestFlags};
use stepper::StepperMotorApparatus;



fn main() {
   let mut stepper = StepperMotorApparatus::new("/dev/gpiochip1","/dev/gpiochip3")
       .expect("StepperMotorApparatus Failed");

    println!("- initialized GPIOs \n");
    println!("- step dt = {:?} us\n", dt);

   loop {
       stepper::StepperMotor
           .set_state(switch = stepper::Switch).await
           .expect("set_state failed");
   }

}
