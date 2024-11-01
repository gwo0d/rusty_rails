use chrono::{
    DateTime,
    Utc,
};

pub struct Departure {
    destination: String,
    scheduled_time: DateTime<Utc>,
    expected_time: Option<DateTime<Utc>>,
    calling_points: Vec<String>,
    platform: Option<u8>,
    status: String,
    delay_reason: Option<String>,
    operator: String,
}

impl Departure {
    pub fn new(destination: String, scheduled_time: DateTime<Utc>, expected_time: Option<DateTime<Utc>>, calling_points: Vec<String>, platform: Option<u8>, status: String, delay_reason: Option<String>, operator: String) -> Self {
        Self { destination, scheduled_time, expected_time, calling_points, platform, status, delay_reason, operator }
    }

    pub fn destination(&self) -> &str {
        &self.destination
    }

    pub fn scheduled_time(&self) -> DateTime<Utc> {
        self.scheduled_time
    }

    pub fn expected_time(&self) -> Option<DateTime<Utc>> {
        self.expected_time
    }

    pub fn calling_points(&self) -> &Vec<String> {
        &self.calling_points
    }

    pub fn platform(&self) -> Option<u8> {
        self.platform
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn delay_reason(&self) -> &Option<String> {
        &self.delay_reason
    }

    pub fn operator(&self) -> &str {
        &self.operator
    }

    pub fn set_expected_time(&mut self, expected_time: Option<DateTime<Utc>>) {
        self.expected_time = expected_time;
    }

    pub fn set_platform(&mut self, platform: Option<u8>) {
        self.platform = platform;
    }

    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }

    pub fn set_delay_reason(&mut self, delay_reason: Option<String>) {
        self.delay_reason = delay_reason;
    }
}