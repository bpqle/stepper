use gpio_cdev::{Chip, LineRequestFlags};

fn main() {
    let mut chip1 = Chip::new("/dev/gpiochip1")
        .expect("Failed to get chip1");
    let mut chip3 = Chip::new("/dev/gpiochip3")
        .expect("Failed to get chip3");

    let motor1_offsets : [u32;2] = [13,12];
    let motor3_offsets : [u32;2] = [19,21];
    let switch_offsets : [u32;2] = [14,15];

    let motor_handle_1 = chip1
        .get_lines(&motor1_offsets)
        .expect("Couldn't get lines 13&12 for motor1")
        .request(LineRequestFlags::OUTPUT, &[1,1], "stepper")
        .expect("Couldn't request lines 13&12 for motor1");
    let motor_handle_3 = chip3
        .get_lines(&motor3_offsets)
        .expect("Couldn't get lines 19&21 for motor3")
        .request(LineRequestFlags::OUTPUT, &[1,1], "stepper")
        .expect("Couldn't request lines 19&21 for motor3");

    let switch_handle_1 = chip1
        .get_lines(&switch_offsets)
        .expect("Couldn't get lines 14&15 for switch")
        .request(LineRequestFlags::INPUT, &[1,1], "switch")
        .expect("Couldn't request lines 14&15 for switch");

    let dt = 1000000 / 500;
    let num_half_steps = 8;
    let mut step_values1: &[u8;2];
    let mut step_values3: &[u8;2];
    let mut step: u8 = 0;

    let all_off : [u8; 2]  = [0,0];
    let half_steps_1 : [[u8;2];8] = [[0,1],[0,1],[0,1],[0,0],[1,0],[1,0],[1,0],[0,0]];
    let half_steps_3 : [[u8;2];8] = [[1,0],[0,0],[0,1],[0,1],[0,1],[0,0],[1,0],[1,0]];

    println!("- initialized GPIOs \n");
    println!("- step dt = {:?} us\n", dt);

    loop {
        let switch_values: Vec<u8> = switch_handle_1.get_values()
            .expect("Failed to get switch values\n");
        if switch_values[0] == 0 {
            step = (step + 1) % &num_half_steps;
            step_values1 = &half_steps_1[usize::from(step)];
            step_values3 = &half_steps_3[usize::from(step)];
        } else if  switch_values[1] == 0 {
            step = (step - 1) % &num_half_steps;
            step_values1 = &half_steps_1[usize::from(step)];
            step_values3 = &half_steps_3[usize::from(step)];
        } else {
            step_values1 = &all_off;
            step_values3 = &all_off;
        }
        motor_handle_1.set_values(step_values1)
            .expect("Cannot set value for motor 1");
        motor_handle_3.set_values(step_values3)
            .expect("Cannot set value for motor 3");
    }

}
