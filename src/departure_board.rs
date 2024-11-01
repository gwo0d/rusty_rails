use crate::departure::Departure;

pub struct DepartureBoard {
    departures: Vec<Departure>,
}

impl DepartureBoard {
    pub fn new(departures: Vec<Departure>) -> Self {
        Self { departures }
    }
}