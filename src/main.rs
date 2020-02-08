use hermes::{generator, Scheduler};

use clap::{App, Arg, ArgMatches};

const NAME: Option<&'static str> = option_env!("CARGO_PKG_NAME");
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const AUTHORS: Option<&'static str> = option_env!("CARGO_PKG_AUTHORS");

fn main() {
    let app = App::new(NAME.unwrap())
        .version(VERSION.unwrap())
        .author(AUTHORS.unwrap())
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .value_name("THREADS")
                .help("Number of worker threads")
                .default_value("3")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("example")
                .short("e")
                .long("example")
                .value_name("EXAMPLE")
                .help("Run the example program. Ignores the generator parameters")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("lanes")
                .short("l")
                .long("lanes")
                .value_name("LANES")
                .help("Number of initial lanes")
                .default_value("4")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("lanes-min")
                .short("m")
                .long("lanes-min")
                .value_name("MIN")
                .help("Number of minimum elements for the initial lanes")
                .default_value("2")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("lanes-dif")
                .short("d")
                .long("lanes-dif")
                .value_name("DIF")
                .help(
                    "Maximum difference between the minimum initial lanes elements and the maximum",
                )
                .default_value("6")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("freq-serial")
                .short("s")
                .long("freq-serial")
                .value_name("SERIAL")
                .help("Frequency in which serial edges are created")
                .default_value("7")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("freq-parallel")
                .short("p")
                .long("freq-parallel")
                .value_name("PARALLEL")
                .help("Frequency in which parallel edges are created")
                .default_value("2")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("freq-fork")
                .short("f")
                .long("freq-fork")
                .value_name("FORK")
                .help("Frequency in which new connections are forked")
                .default_value("7")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("freq-continue")
                .short("c")
                .long("freq-continue")
                .value_name("CONTINUE")
                .help("Frequency in which new connections connects to existing nodes")
                .default_value("2")
                .takes_value(true),
        );

    #[cfg(feature = "trace")]
    let app = app.arg(
        Arg::with_name("log-level")
            .long("log-level")
            .value_name("LOG")
            .possible_values(&["error", "warn", "info", "debug", "trace"])
            .default_value("info")
            .help("Output log level")
            .takes_value(true),
    );

    let matches = app.get_matches();

    #[cfg(feature = "trace")]
    {
        let log = match matches
            .value_of("log-level")
            .expect("Failed parsing log-level arg")
        {
            "error" => tracing::Level::ERROR,
            "warn" => tracing::Level::WARN,
            "info" => tracing::Level::INFO,
            "debug" => tracing::Level::DEBUG,
            "trace" => tracing::Level::TRACE,
            _ => unreachable!(),
        };

        let subscriber = tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(log)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("Failed ton subscribe tracing");
    }

    let threads = arg_to_usize(&matches, "threads");

    let program = if matches.is_present("example") {
        generator::generate_example_program()
    } else {
        generator::generate_program(
            arg_to_usize(&matches, "lanes"),
            arg_to_usize(&matches, "lanes-min"),
            arg_to_usize(&matches, "lanes-dif"),
            arg_to_usize(&matches, "freq-serial"),
            arg_to_usize(&matches, "freq-parallel"),
            arg_to_usize(&matches, "freq-fork"),
            arg_to_usize(&matches, "freq-continue"),
        )
    };

    Scheduler::run(threads, program.as_slice())
        .into_iter()
        .for_each(|s| print!("{}", s));
}

fn arg_to_usize(matches: &ArgMatches, key: &str) -> usize {
    matches
        .value_of(key)
        .expect(format!("Failed to get value of {}", key).as_str())
        .parse()
        .unwrap()
}
