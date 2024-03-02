use std::io;
use std::env;
use std::io::Write;

use ansi_term::Colour::RGB;
use reqwest;
use serde_json;
use tokio;
use goldberg::goldberg_string;

const REFRESH_RATE: u64 = 30;

fn clear_terminal() {
    print!("\x1B[2J\x1B[1;1H");
}

struct Journey {
    due: String,
    expected: String,
    destination: String,
    destination_crs: String,
    platform: String,
    operator: String,
}

impl Journey {
    fn new(due: String, expected: String, destination: String, destination_crs: String, platform: String, operator: String) -> Journey {
        Journey { due, expected, destination, destination_crs, platform, operator }
    }
}

struct DepartureBoard {
    station_crs: String,
    station_name: String,
    departures: Vec<Journey>,
}

impl DepartureBoard {
    fn new(station_crs: String) -> DepartureBoard {
        DepartureBoard { station_crs, station_name: String::new(), departures: Vec::new() }
    }

    fn add_journey(&mut self, journey: Journey) {
        self.departures.push(journey);
    }

    fn clear_departures(&mut self) {
        self.departures.clear();
    }

    async fn get_data_from_api(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .build()?;

        let api_key = String::from(goldberg_string!("<Your API Key>")); // Insert your API key, found here: https://raildata.org.uk/dataProduct/P-9a01dd96-7211-4912-bcbb-c1b5d2e35609/overview

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-apikey", api_key.parse()?);

        let request = client.request(reqwest::Method::GET, format!("https://api1.raildata.org.uk/1010-live-departure-board-dep/LDBWS/api/20220120/GetDepBoardWithDetails/{}?numRows=10&filterCrs=&filterType=to&timeOffset=0&timeWindow=120", self.station_crs))
            .headers(headers);

        let response = request.send().await?;
        let body = response.text().await?;

        let json: serde_json::Value = serde_json::from_str(&body)?;

        self.station_name = json["locationName"].as_str().unwrap().to_string();

        if json["trainServices"].is_null() {
            return Ok(());
        }

        for journey in json["trainServices"].as_array().unwrap() {
            let due = journey["std"].as_str().unwrap().to_string();
            let expected = journey["etd"].as_str().unwrap().to_string();
            let destination = journey["destination"][0]["locationName"].as_str().unwrap().to_string();
            let destination_crs = journey["destination"][0]["crs"].as_str().unwrap().to_string();
            let platform = if journey["platform"].is_null() { "-".to_string() } else { journey["platform"].as_str().unwrap().to_string() };
            let operator_code = journey["operatorCode"].as_str().unwrap().to_string();

            self.add_journey(Journey::new(due, expected, destination, destination_crs, platform, operator_code));
        }

        Ok(())
    }
}

fn format_departure_board(departure_board: &DepartureBoard) -> String {
    let columns = vec![(8, "Due"), (32, "Destination"), (12, "Platform"), (12, "Expected"), (8, "Operator")];
    let mut formatted_board = String::new();
    let text_colour = RGB(255, 130, 60);

    formatted_board.push_str(&text_colour.bold().paint(format!("Departures from {}\n", departure_board.station_name)).to_string());

    for (width, title) in &columns {
        let padding = width - title.len();
        formatted_board.push_str(&text_colour.italic().bold().paint(&**title).to_string());
        formatted_board.push_str(&format!("{:padding$}", "", padding = padding));
    }
    formatted_board.push_str("\n");

    if departure_board.departures.is_empty() {
        formatted_board.push_str(&text_colour.paint("No departures in the next 2 hours.\n").to_string());
        return formatted_board;
    }

    for journey in &departure_board.departures {
        let mut buffer = String::new();
        buffer.push_str(&format!("{}{padding}", journey.due, padding = " ".repeat(columns[0].0 - journey.due.to_string().len())));
        let destination = if journey.destination.to_string().len() > columns[1].0 - 6 { format!("{} ({})", journey.destination.to_string()[0..columns[1].0 - 10].to_string() + "...", journey.destination_crs) } else { format!("{} ({})", journey.destination, journey.destination_crs) };
        buffer.push_str(&format!("{}{padding}", destination, padding = " ".repeat(columns[1].0 - destination.to_string().len())));
        buffer.push_str(&format!("{}{padding}", journey.platform, padding = " ".repeat(columns[2].0 - journey.platform.to_string().len())));
        buffer.push_str(&format!("{}{padding}", journey.expected, padding = " ".repeat(columns[3].0 - journey.expected.to_string().len())));
        buffer.push_str(&format!("{}{padding}", journey.operator, padding = " ".repeat(columns[4].0 - journey.operator.to_string().len())));
        buffer.push_str("\n");

        formatted_board.push_str(&text_colour.paint(buffer).to_string());
    }

    formatted_board
}

struct ArrivalBoard {
    station_crs: String,
    station_name: String,
    arrivals: Vec<Journey>,
}

impl ArrivalBoard {
    fn new(station_crs: String) -> ArrivalBoard {
        ArrivalBoard { station_crs, station_name: String::new(), arrivals: Vec::new() }
    }

    fn add_journey(&mut self, journey: Journey) {
        self.arrivals.push(journey);
    }

    fn clear_arrivals(&mut self) {
        self.arrivals.clear();
    }

    async fn get_data_from_api(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .build()?;

        let api_key = String::from(goldberg_string!("<Your API Key>")); // Insert your API key, found here: https://raildata.org.uk/dataProduct/P-d904019d-1b74-4605-a592-9514883de16f/overview

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("x-apikey", api_key.parse()?);

        let request = client.request(reqwest::Method::GET, format!("https://api1.raildata.org.uk/1010-live-arrival-board-arr/LDBWS/api/20220120/GetArrBoardWithDetails/{}?numRows=5&filterCrs=&filterType=from&timeOffset=0&timeWindow=60", self.station_crs))
            .headers(headers);

        let response = request.send().await?;
        let body = response.text().await?;

        let json: serde_json::Value = serde_json::from_str(&body)?;

        self.station_name = json["locationName"].as_str().unwrap().to_string();

        if json["trainServices"].is_null() {
            return Ok(());
        }

        for journey in json["trainServices"].as_array().unwrap() {
            let due = journey["sta"].as_str().unwrap().to_string();
            let expected = journey["eta"].as_str().unwrap().to_string();
            let destination = journey["origin"][0]["locationName"].as_str().unwrap().to_string();
            let destination_crs = journey["origin"][0]["crs"].as_str().unwrap().to_string();
            let platform = if journey["platform"].is_null() { "-".to_string() } else { journey["platform"].as_str().unwrap().to_string() };
            let operator_code = journey["operatorCode"].as_str().unwrap().to_string();

            self.add_journey(Journey::new(due, expected, destination, destination_crs, platform, operator_code));
        }

        Ok(())
    }
}

fn format_arrival_board(arrival_board: &ArrivalBoard) -> String {
    let columns = vec![(8, "Due"), (32, "Origin"), (12, "Platform"), (12, "Expected"), (8, "Operator")];
    let mut formatted_board = String::new();
    let text_colour = RGB(255, 130, 60);

    formatted_board.push_str(&text_colour.bold().paint(format!("Arrivals at {}\n", arrival_board.station_name)).to_string());

    for (width, title) in &columns {
        let padding = width - title.len();
        formatted_board.push_str(&text_colour.italic().bold().paint(&**title).to_string());
        formatted_board.push_str(&format!("{:padding$}", "", padding = padding));
    }
    formatted_board.push_str("\n");

    if arrival_board.arrivals.is_empty() {
        formatted_board.push_str(&text_colour.paint("No arrivals in the next hour.\n").to_string());
        return formatted_board;
    }

    for journey in &arrival_board.arrivals {
        let mut buffer = String::new();
        buffer.push_str(&format!("{}{padding}", journey.due, padding = " ".repeat(columns[0].0 - journey.due.to_string().len())));
        let origin = if journey.destination.to_string().len() > columns[1].0 - 6 { format!("{} ({})", journey.destination.to_string()[0..columns[1].0 - 10].to_string() + "...", journey.destination_crs) } else { format!("{} ({})", journey.destination, journey.destination_crs) };
        buffer.push_str(&format!("{}{padding}", origin, padding = " ".repeat(columns[1].0 - origin.to_string().len())));
        buffer.push_str(&format!("{}{padding}", journey.platform, padding = " ".repeat(columns[2].0 - journey.platform.to_string().len())));
        buffer.push_str(&format!("{}{padding}", journey.expected, padding = " ".repeat(columns[3].0 - journey.expected.to_string().len())));
        buffer.push_str(&format!("{}{padding}", journey.operator, padding = " ".repeat(columns[4].0 - journey.operator.to_string().len())));
        buffer.push_str("\n");

        formatted_board.push_str(&text_colour.paint(buffer).to_string());
    }

    formatted_board
}

#[tokio::main]
async fn main() {
    let mut crs = String::new();
    let args: Vec<String> = env::args().collect();
    crs = if args.len() > 1 { args[1].clone() } else {
        print!("Enter the CRS code of the station you want to view: ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut crs).unwrap();
        crs.trim().to_string()
    };
    crs = crs.trim().to_string().to_uppercase();

    clear_terminal();

    let mut departure_board = DepartureBoard::new(crs.clone());
    departure_board.get_data_from_api().await.unwrap();

    let mut arrival_board = ArrivalBoard::new(crs.clone());
    arrival_board.get_data_from_api().await.unwrap();

    loop {
        clear_terminal();
        
        println!("{}", format_departure_board(&departure_board));
        println!("{}", format_arrival_board(&arrival_board));
        
        tokio::time::sleep(tokio::time::Duration::from_secs(REFRESH_RATE)).await;

        departure_board.clear_departures();
        departure_board.get_data_from_api().await.unwrap();

        arrival_board.clear_arrivals();
        arrival_board.get_data_from_api().await.unwrap();
    }
}
