use hermes::{generator, Scheduler};

fn main() {
    #[cfg(feature = "trace")]
    {
        let subscriber = tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }

    /*
    Scheduler::run(3, generator::generate_example_program().as_slice())
        .iter()
        .for_each(|s| print!("{}", s));
    */
    Scheduler::run(
        3,
        generator::generate_program(4, 2, 6, 7, 2, 7, 2).as_slice(),
    )
    .iter()
    .for_each(|s| print!("{}", s));
}
