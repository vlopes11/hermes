use hermes::{Operation, Scheduler};

fn main() {
    #[cfg(feature = "trace")]
    {
        let subscriber = tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }

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

    Scheduler::run(3, bytes.as_slice())
        .iter()
        .for_each(|s| print!("{}", s));
}
