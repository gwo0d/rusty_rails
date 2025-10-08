//! # Service Module
//!
//! This module handles all interactions with the National Rail Enquiries API.
//! It defines the data structures for deserializing API responses and provides
//! functions to fetch and process train service boards (departures and arrivals).

use crate::constants::{ARR_BASE_URL, ConfigError, DEP_BASE_URL, arr_api_key, dep_api_key};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::convert::TryFrom;

/// A lazily initialized, shared `reqwest::Client` for making HTTP requests.
/// Using a single client instance is more efficient as it reuses connection pools.
static CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

/// Represents the type of service board to be fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardKind {
    /// A departure board, showing trains leaving a station.
    Departures,
    /// An arrival board, showing trains arriving at a station.
    Arrivals,
}

impl BoardKind {
    /// Returns the display-friendly title for the board type.
    pub fn title(&self) -> &'static str {
        match self {
            BoardKind::Departures => "Departures",
            BoardKind::Arrivals => "Arrivals",
        }
    }

    /// Returns the base URL for the corresponding National Rail API endpoint.
    fn base_url(&self) -> &'static str {
        match self {
            BoardKind::Departures => DEP_BASE_URL,
            BoardKind::Arrivals => ARR_BASE_URL,
        }
    }

    /// Retrieves the appropriate API key from the environment for the board type.
    fn api_key(&self) -> Result<&'static str, ConfigError> {
        match self {
            BoardKind::Departures => dep_api_key(),
            BoardKind::Arrivals => arr_api_key(),
        }
    }
}

/// Internal struct representing the raw JSON response from the API.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    #[serde(default)]
    train_services: Vec<ApiService>,
    location_name: String,
    crs: String,
}

/// Internal struct representing a single train service in the raw API response.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiService {
    destination: Vec<Station>,
    origin: Vec<Station>,
    sta: Option<String>, // Scheduled Time of Arrival
    eta: Option<String>, // Estimated Time of Arrival
    std: Option<String>, // Scheduled Time of Departure
    etd: Option<String>, // Estimated Time of Departure
    operator: String,
    platform: Option<String>,
}

/// Represents a train station with its name, code, and optional routing information.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Station {
    /// The full name of the station (e.g., "London Paddington").
    pub location_name: String,
    /// The 3-letter Computer Reservation System (CRS) code (e.g., "PAD").
    pub crs: String,
    /// Optional routing information, often displayed as "via" a certain station.
    pub via: Option<String>,
}

/// Represents a single train service, cleaned and processed from the raw API data.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    /// The final destination of the service.
    pub destination: Station,
    /// The starting origin of the service.
    pub origin: Station,
    /// Scheduled Time of Arrival.
    pub sta: Option<String>,
    /// Estimated Time of Arrival (e.g., "On time", "10:05", "Delayed").
    pub eta: Option<String>,
    /// Scheduled Time of Departure.
    pub std: Option<String>,
    /// Estimated Time of Departure (e.g., "On time", "10:05", "Cancelled").
    pub etd: Option<String>,
    /// The name of the train operating company.
    pub operator: String,
    /// The platform number, if available.
    pub platform: Option<String>,
}

/// Represents a complete service board for a specific station.
#[derive(Deserialize, Debug)]
pub struct Board {
    /// A list of train services on the board.
    pub services: Vec<Service>,
    /// The name of the station for which the board was fetched.
    pub location_name: String,
    /// The CRS code of the station.
    pub crs: String,
}

/// Safely converts a raw `ApiService` into the application's `Service` model.
///
/// The API returns origin and destination as a list, which may be empty. This
/// conversion fails gracefully if the first (and only expected) station in these
/// lists is missing, ensuring the application only deals with valid services.
impl TryFrom<ApiService> for Service {
    type Error = &'static str;

    fn try_from(api_service: ApiService) -> Result<Self, Self::Error> {
        let destination = api_service
            .destination
            .into_iter()
            .next()
            .ok_or("Missing destination")?;
        let origin = api_service
            .origin
            .into_iter()
            .next()
            .ok_or("Missing origin")?;

        Ok(Service {
            destination,
            origin,
            sta: api_service.sta,
            eta: api_service.eta,
            std: api_service.std,
            etd: api_service.etd,
            operator: api_service.operator,
            platform: api_service.platform,
        })
    }
}

/// Performs the actual HTTP GET request to the National Rail API.
///
/// # Arguments
///
/// * `base_url` - The API base URL for either arrivals or departures.
/// * `api_key` - The API key for authentication.
/// * `station_code` - The CRS code of the station.
/// * `num_rows` - The number of services to request.
async fn fetch_board(
    base_url: &str,
    api_key: &str,
    station_code: &str,
    num_rows: Option<u8>,
) -> Result<Board, reqwest::Error> {
    let url = format!("{}/{}", base_url, station_code.to_uppercase());
    let response = CLIENT
        .get(&url)
        .header("x-apikey", api_key)
        .query(&[("numRows", num_rows.unwrap_or(10))])
        .send()
        .await?
        .json::<ApiResponse>()
        .await?;

    // Convert raw API services to the application's Service model, filtering out any that fail conversion.
    let services = response
        .train_services
        .into_iter()
        .filter_map(|s| Service::try_from(s).ok())
        .collect();

    Ok(Board {
        services,
        location_name: response.location_name,
        crs: response.crs,
    })
}

/// Public interface to fetch a train service board.
///
/// This function retrieves the correct API key and base URL based on the `BoardKind`,
/// then calls `fetch_board` to get the data.
///
/// # Arguments
///
/// * `kind` - The type of board to fetch (`Departures` or `Arrivals`).
/// * `station_code` - The station's CRS code.
/// * `num_rows` - An optional limit for the number of services to return.
///
/// # Errors
///
/// Returns an error if the API key is missing or if the HTTP request fails.
pub async fn try_get_board(
    kind: BoardKind,
    station_code: &str,
    num_rows: Option<u8>,
) -> Result<Board, Box<dyn std::error::Error>> {
    let api_key = kind.api_key()?;
    let board = fetch_board(kind.base_url(), api_key, station_code, num_rows).await?;
    Ok(board)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[test]
    fn board_kind_title() {
        assert_eq!(BoardKind::Departures.title(), "Departures");
        assert_eq!(BoardKind::Arrivals.title(), "Arrivals");
    }

    #[test]
    fn board_kind_base_url() {
        assert_eq!(BoardKind::Departures.base_url(), DEP_BASE_URL);
        assert_eq!(BoardKind::Arrivals.base_url(), ARR_BASE_URL);
    }

    #[test]
    fn service_try_from_api_service_ok() {
        let api_service = ApiService {
            destination: vec![Station {
                location_name: "London Paddington".to_string(),
                crs: "PAD".to_string(),
                via: None,
            }],
            origin: vec![Station {
                location_name: "Reading".to_string(),
                crs: "RDG".to_string(),
                via: None,
            }],
            sta: Some("10:00".to_string()),
            eta: Some("On time".to_string()),
            std: Some("10:05".to_string()),
            etd: Some("On time".to_string()),
            operator: "GWR".to_string(),
            platform: Some("1".to_string()),
        };

        let service = Service::try_from(api_service).unwrap();
        assert_eq!(service.destination.location_name, "London Paddington");
        assert_eq!(service.origin.location_name, "Reading");
    }

    #[test]
    fn service_try_from_api_service_missing_destination() {
        let api_service = ApiService {
            destination: vec![],
            origin: vec![Station {
                location_name: "Reading".to_string(),
                crs: "RDG".to_string(),
                via: None,
            }],
            sta: Some("10:00".to_string()),
            eta: Some("On time".to_string()),
            std: Some("10:05".to_string()),
            etd: Some("On time".to_string()),
            operator: "GWR".to_string(),
            platform: Some("1".to_string()),
        };

        let result = Service::try_from(api_service);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing destination");
    }

    #[test]
    fn service_try_from_api_service_missing_origin() {
        let api_service = ApiService {
            destination: vec![Station {
                location_name: "London Paddington".to_string(),
                crs: "PAD".to_string(),
                via: None,
            }],
            origin: vec![],
            sta: Some("10:00".to_string()),
            eta: Some("On time".to_string()),
            std: Some("10:05".to_string()),
            etd: Some("On time".to_string()),
            operator: "GWR".to_string(),
            platform: Some("1".to_string()),
        };

        let result = Service::try_from(api_service);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Missing origin");
    }

    #[tokio::test]
    async fn fetch_board_success() {
        // Start a mock server.
        let server = MockServer::start();

        // Create a mock for the API endpoint.
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/LBG")
                .header("x-apikey", "fake_api_key")
                .query_param("numRows", "10");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{
                    "locationName": "London Bridge",
                    "crs": "LBG",
                    "trainServices": [
                        {
                            "destination": [{"locationName": "Brighton", "crs": "BTN", "via": null}],
                            "origin": [{"locationName": "London Bridge", "crs": "LBG", "via": null}],
                            "sta": null,
                            "eta": null,
                            "std": "10:00",
                            "etd": "On time",
                            "operator": "Thameslink",
                            "platform": "5"
                        },
                        {
                            "destination": [{"locationName": "Tattenham Corner", "crs": "TAT", "via": null}],
                            "origin": [{"locationName": "London Bridge", "crs": "LBG", "via": null}],
                            "sta": null,
                            "eta": null,
                            "std": "10:05",
                            "etd": "10:07",
                            "operator": "Southern",
                            "platform": "8"
                        }
                    ]
                }"#);
        });

        // Call the function under test.
        let result = fetch_board(&server.base_url(), "fake_api_key", "LBG", Some(10)).await;

        // Assert the mock was called.
        mock.assert();

        // Assert the result is Ok.
        assert!(result.is_ok());
        let board = result.unwrap();

        // Assert the board details are correct.
        assert_eq!(board.location_name, "London Bridge");
        assert_eq!(board.crs, "LBG");
        assert_eq!(board.services.len(), 2);

        // Assert service details are correct.
        assert_eq!(board.services[0].destination.location_name, "Brighton");
        assert_eq!(board.services[0].etd, Some("On time".to_string()));
        assert_eq!(
            board.services[1].destination.location_name,
            "Tattenham Corner"
        );
        assert_eq!(board.services[1].etd, Some("10:07".to_string()));
    }

    #[tokio::test]
    async fn fetch_board_empty_services() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/EMP");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                    "locationName": "Empty Station",
                    "crs": "EMP",
                    "trainServices": []
                }"#,
                );
        });

        let result = fetch_board(&server.base_url(), "fake_api_key", "EMP", None).await;
        mock.assert();

        assert!(result.is_ok());
        let board = result.unwrap();
        assert_eq!(board.location_name, "Empty Station");
        assert_eq!(board.crs, "EMP");
        assert!(board.services.is_empty());
    }

    #[tokio::test]
    async fn fetch_board_filters_invalid_services() {
        let server = MockServer::start();

        let mock = server.mock(|when, then| {
            when.method(GET).path("/MIX");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{
                    "locationName": "Mixed Station",
                    "crs": "MIX",
                    "trainServices": [
                        {
                            "destination": [{"locationName": "Validville", "crs": "VLV", "via": null}],
                            "origin": [{"locationName": "Mixed Station", "crs": "MIX", "via": null}],
                            "std": "11:00",
                            "etd": "On time",
                            "operator": "Good Trains",
                            "platform": "1"
                        },
                        {
                            "destination": [],
                            "origin": [{"locationName": "Mixed Station", "crs": "MIX", "via": null}],
                            "std": "11:05",
                            "etd": "Delayed",
                            "operator": "Bad Trains",
                            "platform": "2"
                        }
                    ]
                }"#);
        });

        let result = fetch_board(&server.base_url(), "fake_api_key", "MIX", None).await;
        mock.assert();

        assert!(result.is_ok());
        let board = result.unwrap();

        // Only the valid service should be present.
        assert_eq!(board.services.len(), 1);
        assert_eq!(board.services[0].destination.location_name, "Validville");
    }
}
