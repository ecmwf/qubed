use qubed::Qube;
use qubed_meteo::adapters::mars_list::FromMARSList;
use std::time::Instant;

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/mars.list");

    let mars_list = std::fs::read_to_string(path).expect("Failed to read MARS list file");

    // let mars_list =
    //     serde_json::from_str::<serde_json::Value>(&mars_list).expect("Failed to parse JSON file");

    let start_time = Instant::now();

    let qube = Qube::from_mars_list(&mars_list);

    // Stop the timer
    let duration = start_time.elapsed();

    // Print the time taken
    println!("Time taken to construct Qube: {:?}", duration);

    // println!("Constructed Qube: {:?}", qube.unwrap().to_ascii());
}
