# Rust Tennis Match Simulator

This repository contains a Rust implementation of a sophisticated tennis match simulator. The simulator models tennis matches between two players, taking into account various factors such as serve probabilities, ace rates, and double fault probabilities.

## Features

- Simulates tennis matches with customizable player statistics
- Supports best-of-3 or best-of-5 set matches
- Implements tiebreak rules, including special rules for final sets in Grand Slams
- Calculates detailed match statistics including aces, double faults, and win probabilities
- Parallel processing for faster simulation of multiple matches
- Exports point-by-point match data to CSV for further analysis

## Requirements

- Rust programming language (latest stable version recommended)
- Cargo package manager

## Dependencies

This project uses the following external crates:

- `serde_json`: For JSON serialization and deserialization
- `rand`: For random number generation
- `rayon`: For parallel processing

Make sure these dependencies are listed in your `Cargo.toml` file.

## Usage

1. Clone this repository:

   ```
   git clone https://github.com/your-username/rust-tennis-simulator.git
   cd rust-tennis-simulator
   ```

2. Build the project:

   ```
   cargo build --release
   ```

3. Run the simulation:
   ```
   cargo run --release
   ```

## Customization

You can customize the simulation by modifying the following parameters in the `main()` function:

- `num_simulations`: Number of matches to simulate
- `num_sets`: Number of sets in each match (3 or 5)
- `max_workers`: Maximum number of parallel workers
- `batch_size`: Number of simulations per batch
- `log_interval`: Interval for saving point-by-point logs
- Player statistics (name, serve win probability, ace probability, double fault probability)

## Output

The simulation provides the following output:

- Percentage of match wins for each player
- Total shots played across all simulations
- Execution time
- Average aces and double faults per match for each player
- Exports a CSV file (`match_log_parallel.csv`) with detailed point-by-point data

## Project Structure

- `main.rs`: Contains the entire simulation code, including player and match structs, simulation logic, and parallel processing implementation

## Contributing

Contributions to improve the simulation model, add new features, or optimize performance are welcome. Please feel free to submit a pull request or open an issue for discussion.

## License

MIT License

## Acknowledgements

This simulator was created as a demonstration of Rust's capabilities for numerical simulations, parallel processing, and statistical analysis in the context of sports modeling.
