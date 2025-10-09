# Rusty Rails

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![GitHub Actions Workflow](https://github.com/gwo0d/rusty_rails/actions/workflows/rust.yml/badge.svg)](https://github.com/gwo0d/rusty_rails/actions/workflows/rust.yml)

A command-line interface (CLI) application for fetching and displaying real-time train departure and arrival information from UK train stations.

This application uses the National Rail Enquiries Darwin API to get live service data. It presents the information in a clean, tabular format and automatically refreshes the data every 15 seconds.

## Features

- **Live Departures and Arrivals**: Get up-to-the-minute train schedules for any UK station.
- **Customizable Display**: Specify the number of rows to display.
- **Auto-Refresh**: The board automatically updates every 15 seconds.
- **Clean Interface**: Displays data in a clean, easy-to-read table.

## Installation

To get started with Rusty Rails, you'll need to have Rust and Cargo installed. You can find instructions at [rust-lang.org](https://www.rust-lang.org/tools/install).

1.  **Clone the repository:**
    ```sh
    git clone https://github.com/gwo0d/rusty_rails.git
    cd rusty-rails
    ```

2.  **Build the project:**
    For the best performance, build the project in release mode.
    ```sh
    cargo build --release
    ```

3.  **Run the application:**
    The executable will be located in the `target/release/` directory.

4.  **Install the executable locally** *(Optional)*:
    For easy access from anywhere in your terminal, you can install the executable to Cargo's bin directory.
    ```sh
    cargo install --path .
    ```

## Usage

To use Rusty Rails, you need to provide a subcommand (`departures` or `arrivals`) and a 3-letter station code (CRS).

### Get Departures

To get the departure board for a station, use the `departures` subcommand (or its aliases `d`, `dep`).

```sh
rusty_rails departures <STATION_CODE>
```

**Example:**
```sh
rusty_rails departures PDM
```

### Get Arrivals

To get the arrival board for a station, use the `arrivals` subcommand (or its aliases `a`, `arr`).

```sh
rusty_rails arrivals <STATION_CODE>
```

**Example:**
```sh
rusty_rails arrivals PDM
```

### Options

-   `-n`, `--num-rows <NUMBER>`: Specify the number of services to display.

**Example:**
```sh
rusty_rails departures PDM -n 10
```

## Configuration

This application requires API keys from National Rail Enquiries. To get your keys, you need to register on the [National Rail Data Portal](https://opendata.nationalrail.co.uk/).

Once you have your API keys, create a `.env` file in the root of the project and add the following lines, replacing the placeholders with your actual keys:

```
DEP_API_KEY=<your Live Departure Board API key>
ARR_API_KEY=<your Live Arrival Board API key>
```

**Note:** These environment variables are also required for running the test suite.

## Development

To contribute to Rusty Rails, you'll need to have Rust and Cargo installed.

### Running Tests

The project includes a full suite of unit and integration tests. To run the tests, use the following command:

```sh
cargo test
```

Make sure you have configured your `.env` file as described in the **Configuration** section, as the tests require the API keys to be present.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
