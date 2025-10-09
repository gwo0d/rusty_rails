//! # Rusty Rails
//!
//! A command-line interface (CLI) application for fetching and displaying
//! real-time train departure and arrival information from UK train stations.
//!
//! This application uses the National Rail Enquiries Darwin API to get live
//! service data. It presents the information in a clean, tabular format and
//! automatically refreshes the data periodically.

use clap::Parser;
use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
    modifiers::{UTF8_ROUND_CORNERS, UTF8_SOLID_INNER_BORDERS},
    presets::UTF8_FULL,
};
use dotenvy::dotenv;
use service::{Board, BoardKind, Service, Station};
use std::time::Duration;
use tokio::{signal, time};

mod constants;
mod error;
mod service;

use error::AppError;

/// The interval in seconds at which the train service board will automatically refresh.
const REFRESH_INTERVAL_SECS: u64 = 15;

/// Defines the command-line arguments for the Rusty Rails application.
///
/// This struct uses `clap` to parse and validate command-line arguments. It
/// defines the main command structure, including subcommands for `departures`
/// and `arrivals`, and an optional argument for the number of rows to display.
#[derive(Parser, Debug)]
#[command(
    name = "rusty_rails",
    author = "George O. Wood",
    version = "2.1.4",
    about = "A CLI for fetching train departure and arrival boards.",
    long_about = None
)]
struct Cli {
    /// The specific command to execute (e.g., departures or arrivals).
    #[command(subcommand)]
    command: Commands,

    /// Optional: The number of rows (services) to display in the board.
    #[arg(short, long, help = "Number of rows to display.")]
    num_rows: Option<u8>,
}

/// Enumerates the available subcommands for the CLI.
///
/// This enum defines the `departures` and `arrivals` subcommands, each of which
/// requires a `station_code` argument. It also specifies aliases for convenience.
#[derive(Parser, Debug)]
enum Commands {
    /// Fetches and displays the departure board for a given station.
    #[command(name = "departures", visible_aliases = ["d", "dep"])]
    Departures {
        /// The 3-letter station code (CRS) to get departures for.
        #[arg(help = "The station code to get departures for.")]
        station_code: String,
    },
    /// Fetches and displays the arrival board for a given station.
    #[command(name = "arrivals", visible_aliases = ["a", "arr"])]
    Arrivals {
        /// The 3-letter station code (CRS) to get arrivals for.
        #[arg(help = "The station code to get arrivals for.")]
        station_code: String,
    },
}

/// Creates and configures a new `comfy_table::Table` with default styling.
///
/// This function initializes a new table with UTF-8 presets for borders and
/// corners, and styles the headers to be bold and center-aligned.
///
/// # Arguments
///
/// * `headers` - A vector of string slices that will be used as the table headers.
///
/// # Returns
///
/// A `Table` instance ready for content to be added.
fn create_table(headers: Vec<&str>) -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .apply_modifier(UTF8_SOLID_INNER_BORDERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers.into_iter().map(|h| {
            Cell::new(h)
                .add_attribute(Attribute::Bold)
                .set_alignment(CellAlignment::Center)
        }));
    table
}

/// Formats station information, including an optional "via" text.
///
/// # Arguments
///
/// * `station` - A reference to a `Station` struct containing location details.
///
/// # Returns
///
/// A formatted `String` in the format "Location Name (CRS)" with an optional
/// "via" line if present.
///
/// # Example
///
/// ```
/// use rusty_rails::service::Station;
///
/// let station_with_via = Station {
///     location_name: "Gatwick Airport".to_string(),
///     crs: "GTW".to_string(),
///     via: Some("via Redhill".to_string()),
/// };
/// assert_eq!(format_station(&station_with_via), "Gatwick Airport (GTW)
/// via Redhill");
///
/// let station_without_via = Station {
///     location_name: "London Victoria".to_string(),
///     crs: "VIC".to_string(),
///     via: None,
/// };
/// assert_eq!(format_station(&station_without_via), "London Victoria (VIC)");
/// ```
fn format_station(station: &Station) -> String {
    let mut result = format!("{} ({})", station.location_name, station.crs);
    if let Some(via) = &station.via {
        result.push_str(&format!("\n{via}"));
    }
    result
}

/// Applies color to the expected time cell based on its content.
///
/// "On time" is colored green, while any other status (e.g., "Delayed", "Cancelled",
/// or a specific time) is colored red. This provides a quick visual cue for the
/// status of a service.
///
/// # Arguments
///
/// * `expected` - A string slice representing the expected time or status.
///
/// # Returns
///
/// A `Cell` with appropriate color and styling.
fn colourise_expected(expected: &str) -> Cell {
    let color = if expected.eq_ignore_ascii_case("On time") {
        Color::Green
    } else {
        Color::Red
    };
    Cell::new(expected)
        .add_attribute(Attribute::Bold)
        .set_alignment(CellAlignment::Center)
        .fg(color)
}

/// Prints a list of train services to the console in a formatted table.
///
/// This function constructs and prints a table of train services. The first
/// column of the table is context-dependent: it shows "Destination" for a
/// departure board and "Origin" for an arrival board.
///
/// # Arguments
///
/// * `services` - A vector of `Service` structs to be displayed.
/// * `kind` - The type of board (`Departures` or `Arrivals`), which determines
///   the table layout and content.
fn print_services(services: &[Service], kind: BoardKind) {
    let is_departures = matches!(kind, BoardKind::Departures);
    let headers = if is_departures {
        vec![
            "Destination",
            "Platform",
            "Operator",
            "Scheduled",
            "Expected",
        ]
    } else {
        vec!["Origin", "Platform", "Operator", "Scheduled", "Expected"]
    };
    let mut table = create_table(headers);

    for service in services {
        // Destructure service details based on whether it's a departure or arrival.
        let (station_cell, scheduled_time, expected_time) = if is_departures {
            (
                Cell::new(format_station(&service.destination)),
                service.std.as_deref().unwrap_or_default(),
                service.etd.as_deref().unwrap_or_default(),
            )
        } else {
            (
                Cell::new(format_station(&service.origin)),
                service.sta.as_deref().unwrap_or_default(),
                service.eta.as_deref().unwrap_or_default(),
            )
        };

        table.add_row(vec![
            station_cell,
            Cell::new(service.platform.as_deref().unwrap_or("--"))
                .set_alignment(CellAlignment::Center),
            Cell::new(&service.operator).set_alignment(CellAlignment::Center),
            Cell::new(scheduled_time).set_alignment(CellAlignment::Center),
            colourise_expected(expected_time),
        ]);
    }

    println!("{table}");

    // Print exit/refresh instructions.
    println!(
        "[1m[3mAuto-refreshing every {}s. Press Ctrl+C to exit.[0m",
        REFRESH_INTERVAL_SECS
    );
}

/// Clears the screen and prints the given board details.
///
/// This function handles the presentation logic. It clears the terminal,
/// displays a message if no services are available, or prints a formatted
/// table of services.
///
/// # Arguments
///
/// * `board` - A reference to the `Board` data to be displayed.
/// * `kind` - The type of board (`Departures` or `Arrivals`).
/// * `station_code` - The station code (CRS) used for the query.
///
/// # Errors
///
/// Returns an error if clearing the screen fails.
fn print_board_details(board: &Board, kind: BoardKind, station_code: &str) -> Result<(), AppError> {
    // Clear the terminal screen before printing the new board.
    clearscreen::clear()?;

    if board.services.is_empty() {
        println!("No services found for station code '{station_code}'.");
    } else {
        // Print the board header.
        println!(
            "{} for {} ({})",
            kind.title(),
            board.location_name,
            board.crs
        );
        println!("Last updated: {}", chrono::Local::now().format("%H:%M:%S"));
        println!();

        // Print the services in a table.
        print_services(&board.services, kind);
    }

    Ok(())
}

/// The main entry point for the application.
///
/// This function initializes the application by performing the following steps:
/// 1. Loads environment variables from a `.env` file.
/// 2. Validates required API keys if the `fail-fast-config` feature is enabled.
/// 3. Parses command-line arguments to determine the station and board type.
/// 4. Performs an initial fetch and print of the service board.
/// 5. Enters a main loop that listens for user input and periodically refreshes
///    the data. The loop exits when any key is pressed.
#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Load environment variables from a .env file, if it exists.
    let _ = dotenv();

    // If the `fail-fast-config` feature is enabled, validate required environment
    // variables at startup and exit if any are missing.
    #[cfg(feature = "fail-fast-config")]
    {
        if let Err(e) = crate::constants::validate_required_keys() {
            eprintln!("Configuration error: {e}");
            std::process::exit(1);
        }
    }

    // Parse command-line arguments.
    let cli = Cli::parse();
    let (station_code, kind) = match cli.command {
        Commands::Departures { station_code } => (station_code, BoardKind::Departures),
        Commands::Arrivals { station_code } => (station_code, BoardKind::Arrivals),
    };

    let num_rows = cli.num_rows;

    // Perform the initial fetch and print.
    let board = service::try_get_board(kind, &station_code, num_rows).await?;
    print_board_details(&board, kind, &station_code)?;

    // Set up a timer for periodic refreshes.
    let mut interval = time::interval(Duration::from_secs(REFRESH_INTERVAL_SECS));

    // Main application loop.
    // This loop uses `tokio::select!` to concurrently listen for two events:
    // 1. A tick from the refresh interval.
    // 2. A Ctrl+C signal from the user.
    // The loop terminates immediately when a Ctrl+C signal is received.
    loop {
        tokio::select! {
            // Listen for the Ctrl+C signal.
            _ = signal::ctrl_c() => {
                // The signal future resolves, so we break the loop to exit.
                break;
            }
            // Wait for the refresh interval timer to tick.
            _ = interval.tick() => {
                // Fetch the latest service board data.
                match service::try_get_board(kind, &station_code, num_rows).await {
                    Ok(board) => {
                        if let Err(e) = print_board_details(&board, kind, &station_code) {
                            eprintln!("Error printing board: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error refreshing services: {}", e);
                    }
                }
            }
        }
    }

    println!("\nExiting...");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::Station;
    use comfy_table::{Attribute, Cell, CellAlignment, Color};

    #[test]
    fn test_format_station_no_via() {
        let station = Station {
            location_name: "London Victoria".to_string(),
            crs: "VIC".to_string(),
            via: None,
        };
        assert_eq!(format_station(&station), "London Victoria (VIC)");
    }

    #[test]
    fn test_format_station_with_via() {
        let station = Station {
            location_name: "Gatwick Airport".to_string(),
            crs: "GTW".to_string(),
            via: Some("via Redhill".to_string()),
        };
        let expected = "Gatwick Airport (GTW)
via Redhill";
        assert_eq!(format_station(&station), expected);
    }

    #[test]
    fn test_colourise_expected_on_time() {
        let actual_cell = colourise_expected("On time");
        let expected_cell = Cell::new("On time")
            .add_attribute(Attribute::Bold)
            .set_alignment(CellAlignment::Center)
            .fg(Color::Green);
        assert_eq!(actual_cell, expected_cell);
    }

    #[test]
    fn test_colourise_expected_delayed() {
        let actual_cell = colourise_expected("Delayed");
        let expected_cell = Cell::new("Delayed")
            .add_attribute(Attribute::Bold)
            .set_alignment(CellAlignment::Center)
            .fg(Color::Red);
        assert_eq!(actual_cell, expected_cell);
    }

    #[test]
    fn test_colourise_expected_cancelled() {
        let actual_cell = colourise_expected("Cancelled");
        let expected_cell = Cell::new("Cancelled")
            .add_attribute(Attribute::Bold)
            .set_alignment(CellAlignment::Center)
            .fg(Color::Red);
        assert_eq!(actual_cell, expected_cell);
    }

    #[test]
    fn test_colourise_expected_numerical_time() {
        let actual_cell = colourise_expected("10:15");
        let expected_cell = Cell::new("10:15")
            .add_attribute(Attribute::Bold)
            .set_alignment(CellAlignment::Center)
            .fg(Color::Red);
        assert_eq!(actual_cell, expected_cell);
    }
}
