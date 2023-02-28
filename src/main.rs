use stepper::{StepperMotorApparatus};
use std::time::Duration;
use std::thread;
use simple_logger::SimpleLogger;
use log::{trace, info, warn};



#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();
   let stepper = StepperMotorApparatus::new("/dev/gpiochip1", "/dev/gpiochip3")
       .expect("StepperMotorApparatus Failed");
    info!("Apparatus created");
    stepper.switch_ctrl().await.unwrap();
    info!("Switch Control started");
    loop {
        info!("Main thread reporting");
        thread::sleep(Duration::from_secs(10));
    }

}
