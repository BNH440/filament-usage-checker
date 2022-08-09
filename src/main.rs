use std::f32::consts::PI;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use progressing::{
    Baring,
    clamping::Bar as ClampingBar,
};
use prettytable::format;

#[macro_use] extern crate prettytable;
use prettytable::{Table};

#[derive(Serialize, Deserialize, Debug)]
struct Thumbnail {
    pub width: i64,
    pub height: i64,
    pub size: i64,
    pub relative_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Metadata {
    pub size: i64,
    pub modified: f64,
    pub uuid: String,
    pub slicer: String,
    pub slicer_version: String,
    pub gcode_start_byte: i64,
    pub gcode_end_byte: i64,
    pub layer_count: i64,
    pub object_height: f64,
    pub estimated_time: i64,
    pub nozzle_diameter: f64,
    pub layer_height: f64,
    pub first_layer_height: f64,
    pub first_layer_extr_temp: f64,
    pub first_layer_bed_temp: f64,
    pub filament_name: String,
    pub filament_type: String,
    pub filament_total: f64,
    pub filament_weight_total: f64,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Job {
    pub end_time: f64,
    pub filament_used: f64,
    pub filename: String,
    pub metadata: Metadata,
    pub print_duration: f64,
    pub status: String,
    pub start_time: f64,
    pub total_duration: f64,
    pub job_id: String,
    pub exists: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Result {
    pub count: i64,
    pub jobs: Vec<Job>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    pub result: Result,
}


const IP: &str = "192.168.1.8";
const PORT: &str = "8787";
const PLA_DENSITY: f64 = 1.25;
const PLA_RADIUS: f64 = 0.175 / 2.0;


fn meters_to_grams(meters: f64) -> f64 {
    let cm: f64 = meters * 100.0;
    let volume: f64 = ((PI) as f64) * PLA_RADIUS.powf(2.0) * cm; // pi * r^2 * l

    return volume * PLA_DENSITY;
}

#[async_std::main]
async fn main() {
    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());

    app.at("/").get(|_| async { Ok(parse().await) });

    println!("Listening on http://{}:{}", IP, PORT);
    app.listen(format!("0.0.0.0:{}", PORT)).await.unwrap();
}

async fn parse() -> String {
    let spools = get_spool_data().await;

    let mut v: Vec<_> = spools.into_iter().collect();
    v.sort_by(|x,y| x.0.cmp(&y.0));

    let mut table = Table::new();
    table.add_row(row!["Spool Name", "Usage (g)", "Remaining (g)", "% Used"]);

    v.iter().for_each(|(name, grams)| {
        let mut progress_bar = ClampingBar::new();
        progress_bar.set_len(20);
        progress_bar.set(grams.round() / 1000.0);
        table.add_row(row![name, grams.round(), (1000.0 - grams).round(), format!("{} {}%", progress_bar, ((grams / 1000.0) * 100.0).round())]);
    });

    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    return table.to_string();
}

async fn get_spool_data() -> HashMap<String, f64> {
    let mut filament_rolls = HashMap::from([
        (String::from("White Spool"), 823.0),
        (String::from("New Black Spool"), 893.0),
        (String::from("Kevin's Spool"), 958.0),
        (String::from("Grey Spool"), 710.0),
        (String::from("Blue Spool"), 870.0),
        (String::from("Black Spool"), 977.0),
        (String::from("Orange Spool"), 63.0),
    ]);

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{IP}/server/history/list?start=108&order=asc"))
        .send()
        .await
        .unwrap();

    println!("Request: {}", response.status());

    match response.status() {
        reqwest::StatusCode::OK => {
            match response.json::<ApiResponse>().await {
                Ok(parsed) => {
                    parsed.result.jobs.iter().for_each(|job| {
                        if filament_rolls.contains_key(job.metadata.filament_name.as_str()) {
                            let filament_roll = filament_rolls.get_mut(job.metadata.filament_name.as_str()).unwrap();
                            *filament_roll += meters_to_grams(job.filament_used / 1000.0);
                        } else {
                            filament_rolls.insert(job.metadata.filament_name.as_str().parse().unwrap(), meters_to_grams(job.filament_used / 1000.0));
                        }
                    });
                },
                Err(_) => println!("Error parsing JSON"),
            };
        }
        other => {
            panic!("Error: {:?}", other);
        }
    };

    return filament_rolls;
}
