use crate::{Operation, Scheduler, Task};

use rand::Rng;

pub fn generate_program(
    initial_lanes: usize,
    initial_qty_min: usize,
    initial_qty_dif: usize,
    freq_serial: usize,
    freq_parallel: usize,
    freq_parallel_new_lane: usize,
    freq_parallel_existing_lane: usize,
) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let base_lanes = (0..initial_lanes)
        .map(|_| rng.gen_range(initial_qty_min, initial_qty_min + initial_qty_dif))
        .map(|qty| {
            (0..qty)
                .map(|_| rand::thread_rng().gen::<f64>())
                .map(|r| r + if flip_frequency(1, 2) { 1.0f64 } else { 0.1f64 })
                .collect::<Vec<f64>>()
        })
        .enumerate()
        .map(|(i, v)| ((i + 1) as u32, v))
        .collect::<Vec<(u32, Vec<f64>)>>();

    let mut bytes = Scheduler::lane_initializer(base_lanes.clone());

    let mut lane_idx = (base_lanes.len() + 1) as u32;
    let mut frontier = base_lanes
        .into_iter()
        .map(|(i, v)| (i, v.len() as u32))
        .collect::<Vec<(u32, u32)>>();

    while let Some((lane, size)) = frontier.pop() {
        let mut task = generate_serial(lane, size, freq_serial, freq_parallel);

        let should_fork = flip_frequency(freq_parallel_new_lane, freq_parallel_existing_lane);

        if frontier.is_empty() {
            task.push(Operation::Return);
        } else if should_fork {
            lane_idx += 1;
            task.push(Operation::Store(lane_idx));
            frontier.push((lane_idx, 1));
        } else if !should_fork {
            let idx = rng.gen_range(0, frontier.len());
            task.push(Operation::Store(frontier[idx].0));
            frontier[idx].1 += 1;
        }

        write_task(&mut bytes, task.into());
    }

    bytes
}

pub fn generate_example_program() -> Vec<u8> {
    let mut bytes = Scheduler::lane_initializer(vec![
        (1, vec![2.0, 3.0, 5.0]),
        (2, vec![2.0, 3.0, 5.0]),
        (3, vec![2.0, 3.0, 5.0]),
        (4, vec![2.0]),
    ]);

    bytes.extend_from_slice(&Operation::Add(1, 3).to_vec());
    bytes.extend_from_slice(&Operation::Exp.to_vec());
    bytes.extend_from_slice(&Operation::Store(5).to_vec());
    bytes.extend_from_slice(&Operation::Add(2, 3).to_vec());
    bytes.extend_from_slice(&Operation::Store(5).to_vec());
    bytes.extend_from_slice(&Operation::Add(3, 3).to_vec());
    bytes.extend_from_slice(&Operation::Pow(2.0).to_vec());
    bytes.extend_from_slice(&Operation::Store(5).to_vec());
    bytes.extend_from_slice(&Operation::Mul(5, 3).to_vec());
    bytes.extend_from_slice(&Operation::Return.to_vec());

    bytes
}

pub fn generate_serial(
    lane: u32,
    size: u32,
    freq_serial: usize,
    freq_parallel: usize,
) -> Vec<Operation> {
    let mut ops = vec![];

    ops.push(Operation::random_parallel(lane, size));

    while flip_frequency(freq_serial, freq_parallel) {
        ops.push(Operation::random_serial());
    }

    ops
}

fn flip_frequency(a: usize, b: usize) -> bool {
    rand::thread_rng().gen_range(0, a + b) < a
}

fn write_task(bytes: &mut Vec<u8>, task: Task) {
    for op in task.ops {
        bytes.extend_from_slice(&op.to_vec());
    }
}
