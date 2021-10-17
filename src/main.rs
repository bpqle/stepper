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
    stepper.lock().unwrap()
        .switch.switch_ctrl(
            &mut stepper.try_lock().unwrap().stepper_motor).await.unwrap();

   loop {
      println!("busy work in main thread");
       thread::sleep(Duration::from_secs(10));
   }

}
