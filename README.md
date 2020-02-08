# Hermes

Hermes execute DAG graphs with parallel workers.

# Internals

## Fifo work queue

The structure allows the user to `push` data, return an `Option<T>` in case the queue is full, and `pop` data, returning also an `Option<T>`, considering if there is any data on the queue.

* Data
Slice of tasks to be done.

* Head pointer
Position of the next pending task. Protected by a `Mutex` due to the intensive read/write. It is consumed by the workers

* Tail pointer
Position of the last task. The read/write operations follow an `Acquire` / `Release` pattern. Consumed internally

* Len
Number of elements on the queue. The read/write operations follow an `Acquire` / `Release` pattern. There is an unsync public len for external consumers.

## Scheduler

The scheduler will receive instructions from a `mpsc::channel` and push tasks to the `Fifo` queue, fetch / update the data lanes, and evaluate the execution of the program.

It will spawn several workers that will consume the contents of the `Fifo` and send the result to the `mpsc::channel`. If a worker try to fetch data from a not ready lane, then the current work will be sent as pending to the `mpsc::channel` and the worker will try to fetch another job.

Every work execution must begin with a successful `Fetch` from a provided lane.

* Fifo
The pending tasks to be executed

* Workers
Open threads to execute the tasks. If there is no available work, they are put to sleep with `thread::park()`. When new work is pushed, the sleeping workers are awakened.

* Task buffer
In case the FIFO is full, the tasks are stored on the buffer for future rescheduling.

* Data lanes
Indexed map to refer sets of data to be used in multiple parameter operation execution.

* Parser
From an implementation of `io::Read`, consume the available bytes to build set of operations denominated as `Task`.

A task is considered serial until there is either a commando `Store` to send the processed data to a lane, or `Return` to finish the overall processing.

Every task begins with a `Fetch` command, that will extract `n` elements from a given lane.

After the task is parsed, it is sent to the Scheduler via a `mpsc::channel`

# Usage

![hermes](https://user-images.githubusercontent.com/8730839/74088499-7748d380-4a97-11ea-85dc-ebc673b112d9.gif)

## Requirements

* [Rust Nightly](https://www.rust-lang.org/tools/install)

## Instructions

Clone the github repo

```
$ git clone https://github.com/vlopes11/hermes.git
$ cd hermes
```

Test the build

`$ cargo test --release`

Build the binaries

`$ cargo build --release`

Build the binaries with log tracing

`$ cargo build --release --features trace`

## Options

```
USAGE:
    hermes [FLAGS] [OPTIONS]

FLAGS:
    -e, --example    Run the example program. Ignores the generator parameters
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --freq-continue <CONTINUE>    Frequency in which new connections connects to existing nodes [default: 2]
    -f, --freq-fork <FORK>            Frequency in which new connections are forked [default: 7]
    -p, --freq-parallel <PARALLEL>    Frequency in which parallel edges are created [default: 2]
    -s, --freq-serial <SERIAL>        Frequency in which serial edges are created [default: 7]
    -l, --lanes <LANES>               Number of initial lanes [default: 4]
    -d, --lanes-dif <DIF>             Maximum difference between the minimum initial lanes elements and the maximum
                                      [default: 6]
    -m, --lanes-min <MIN>             Number of minimum elements for the initial lanes [default: 2]
    -t, --threads <THREADS>           Number of worker threads [default: 3]
```

## Example

The following DAG will be executed

![DAG](https://user-images.githubusercontent.com/8730839/74088510-892a7680-4a97-11ea-8d54-fad6d9094cfb.png)

`$ ./target/release/hermes --example`
