use crate::departure::Departure;

pub struct DepartureBoard {
    departures: Vec<Departure>,
}

impl DepartureBoard {
    pub fn new() -> Self {
        Self { departures: Vec::new() }
    }

    pub fn add_departure(&mut self, departure: Departure) {
        self.departures.push(departure);
        self.sort_by_eta()
    }

    pub fn remove_departure(&mut self, index: usize) {
        self.departures.remove(index);
        self.sort_by_eta()
    }

    pub fn print_departures(&self) {
        for departure in self.departures.iter() {
            println!("{}\n", departure.summarise_to_string())
        }
    }

    fn sort_by_eta(&mut self) {
        self.departures.sort_by(|a, b| a.eta().timestamp().cmp(&b.eta().timestamp()))
    }
}