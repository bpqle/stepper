use gpio_cdev::{Chip, LineRequestFlags};
use stepper::StepperMotorApparatus;



fn main() {
   let mut motor = StepperMotorApparatus::new("/dev/gpiochip1","/dev/gpiochip3")
       .expect("Couldn't get either motors");





    println!("- initialized GPIOs \n");
    println!("- step dt = {:?} us\n", dt);

    loop {
        let switch_values: Vec<u8> = switch_handle_1.get_values()
            .expect("Failed to get switch values\n");
        if switch_values[0] == 0 {
            step = (step + 1) % &num_half_steps;
            step_values1 = &half_steps[step].0;
            step_values3 = &half_steps[step].1;
        } else if  switch_values[1] == 0 {
            step = (step - 1) % &num_half_steps;
            step_values1 = &half_steps[step].0;
            step_values3 = &half_steps[step].1;
        } else {
            step_values1 = &all_off;
            step_values3 = &all_off;
        }
        motor_handle_1.set_values(&step_values1.0)
            .expect("Cannot set value for motor 1");
        motor_handle_3.set_values(&step_values3.0)
            .expect("Cannot set value for motor 3");
    }

}
