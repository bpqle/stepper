use gpio_cdev::{Chip, LineRequestFlags, EventRequestFlags, EventType};

fn main() {
    let mut chip1 = Chip::new("/dev/gpiochip1");
    let mut chip3 = Chip::new("/dev/gpiochip3");

    let motor_handle_1 = chip1
        .get_lines(vec![13, 12])
        .request(LineRequestFlags::OUTPUT, 0, "stepper")?;
    let motor_handle_3 = chip3
        .get_lines(vec![19, 21])
        .request(LineRequestFlags::OUTPUT, 0, "stepper")?;

    let switch_handle_1 = chip1
        .get_lines(vec![14, 15])
        .request(LineRequestFlags::INPUT, 0, "switch")?;

    let dt = 1000000 / 500;
    let num_half_steps = 8;
    let mut step_values;
    let mut step: u8 = 0;


    println!("- initialized GPIOs \n");
    println!("- step dt = {:?} us\n", dt);

    loop {
        let mut switch_values = switch_handle_1.get_values()?;
        if switch_values[0] == 0 {
            step = (step + 1) % &num_half_steps;
            step_values = half_steps[&step];
        } else if  switch_values[1] == 0 {
            step = (step - 1) % &num_half_steps;
            step_values = half_steps[&step];
        } else {
            step_values = all_off;
        }
        motor_handle_1.set_values(&step_values);
        motor_handle_3.set_values(&step_values);
    }

}
