//! # Constants and Configuration Module
//!
//! This module defines static constants and handles the loading and validation
//! of configuration from environment variables. It is responsible for providing
//! API base URLs and ensuring that the necessary API keys are available.
//!
//! API keys are loaded lazily and cached on their first use to improve performance
//! and avoid repeated environment lookups.

use once_cell::sync::OnceCell;
use std::env;
use std::fmt;

/// Base URL for the National Rail Live Departure Board API.
pub const DEP_BASE_URL: &str = "https://api1.raildata.org.uk/1010-live-departure-board-dep1_2/LDBWS/api/20220120/GetDepartureBoard";
/// Base URL for the National Rail Live Arrival Board API.
pub const ARR_BASE_URL: &str = "https://api1.raildata.org.uk/1010-live-arrival-board-arr1_1/LDBWS/api/20220120/GetArrivalBoard";

/// Represents errors that can occur when loading configuration from environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// The specified environment variable is not set.
    MissingVar(&'static str),
    /// The environment variable is set but contains an empty or whitespace-only value.
    EmptyVar(&'static str),
}

impl fmt::Display for ConfigError {
    /// Formats the configuration error for display.
    ///
    /// This implementation provides a user-friendly error message explaining
    /// which environment variable is missing or empty and how to fix it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rusty_rails::constants::ConfigError;
    ///
    /// let missing_error = ConfigError::MissingVar("API_KEY");
    /// assert_eq!(
    ///     missing_error.to_string(),
    ///     "Required environment variable 'API_KEY' is not set. \
    ///      Provide it in your shell or a .env file."
    /// );
    ///
    /// let empty_error = ConfigError::EmptyVar("API_KEY");
    /// assert_eq!(
    ///     empty_error.to_string(),
    ///     "Environment variable 'API_KEY' is set but empty. \
    ///      It must contain a non-empty API key."
    /// );
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingVar(var) => write!(
                f,
                "Required environment variable '{var}' is not set. \
                Provide it in your shell or a .env file."
            ),
            ConfigError::EmptyVar(var) => write!(
                f,
                "Environment variable '{var}' is set but empty. \
                It must contain a non-empty API key."
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

/// A thread-safe, write-once cell to cache the departures API key.
static DEP_API_KEY_CELL: OnceCell<String> = OnceCell::new();
/// A thread-safe, write-once cell to cache the arrivals API key.
static ARR_API_KEY_CELL: OnceCell<String> = OnceCell::new();

/// A generic helper function to lazily load, validate, and cache a configuration value.
///
/// It uses a `OnceCell` to ensure the `fetch` closure is only executed once.
/// On the first call, it runs the closure, validates that the result is not empty,
/// and stores it in the cell. Subsequent calls return the cached value directly.
///
/// # Arguments
///
/// * `var` - The name of the variable being loaded (for error reporting).
/// * `cell` - The `OnceCell` used for caching.
/// * `fetch` - A closure that attempts to load the value.
fn load_with<F>(
    var: &'static str,
    cell: &'static OnceCell<String>,
    fetch: F,
) -> Result<&'static str, ConfigError>
where
    F: for<'a> Fn(&'a str) -> Result<String, std::env::VarError>,
{
    let value_ref = cell.get_or_try_init(|| match fetch(var) {
        Ok(val) => {
            if val.trim().is_empty() {
                Err(ConfigError::EmptyVar(var))
            } else {
                Ok(val)
            }
        }
        Err(_) => Err(ConfigError::MissingVar(var)),
    })?;
    Ok(value_ref.as_str())
}

/// Loads a variable from the environment and caches it using the `load_with` helper.
fn load_and_cache(
    var: &'static str,
    cell: &'static OnceCell<String>,
) -> Result<&'static str, ConfigError> {
    load_with(var, cell, |s| env::var(s))
}

/// Retrieves the departures API key (`DEP_API_KEY`) from the environment.
///
/// The key is loaded on the first call and cached for subsequent access.
///
/// # Errors
///
/// Returns `ConfigError` if the key is missing or empty.
pub fn dep_api_key() -> Result<&'static str, ConfigError> {
    load_and_cache("DEP_API_KEY", &DEP_API_KEY_CELL)
}

/// Retrieves the arrivals API key (`ARR_API_KEY`) from the environment.
///
/// The key is loaded on the first call and cached for subsequent access.
///
/// # Errors
///
/// Returns `ConfigError` if the key is missing or empty.
pub fn arr_api_key() -> Result<&'static str, ConfigError> {
    load_and_cache("ARR_API_KEY", &ARR_API_KEY_CELL)
}

/// Eagerly validates that all required API keys are present and valid.
///
/// This function is intended to be called at application startup to "fail fast"
/// if the necessary configuration is not provided.
///
/// # Errors
///
/// Returns `ConfigError` if any key is missing or empty.
pub fn validate_required_keys() -> Result<(), ConfigError> {
    dep_api_key()?;
    arr_api_key()?;
    Ok(())
}
