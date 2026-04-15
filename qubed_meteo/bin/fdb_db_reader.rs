use qubed::Qube;
use qubed_meteo::adapters::fdb::FromFDBList;
use rsfdb::{FDB, request::Request};
use serde_json::json;
use std::env;
use std::fs::File;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure FDB config is set so the internal listing can open the DB
    use std::path::PathBuf;

    let config_path = PathBuf::from("/home/eouser/male/qubed_fdb_gen/config/fdb_config.yaml"); // Adjust this path to point to your local FDB config.yaml
    unsafe {
        std::env::set_var("FDB5_CONFIG_FILE", config_path.to_str().expect("Invalid config path"));
    }

    let lib_path = PathBuf::from("/home/eouser/male/qubed_fdb_gen/gribjump_bundle/build/"); // Adjust this path to point to the directory containing FDB shared libraries

    unsafe {
        std::env::set_var(
            "DYLD_LIBRARY_PATH",
            lib_path.to_str().expect("Invalid path to shared libraries"),
        );
    }

    let request_map = json!({
        "class" : "d1",
        "dataset": "extremes-dt",
        "expver" : "0001",
        "stream" : "oper",
        "levtype" : "sfc",
        "date": "20260410",
    });
    let start_time = Instant::now();

    // Build the Qube directly from the request; the adapter will open FDB and list.
    let qube = Qube::from_fdb_list(&request_map).expect("Failed to build Qube from FDB list");

    // Stop the timer
    let duration = start_time.elapsed();

    // Print the time taken
    println!("Time taken to construct Qube: {:?}", duration);

    let file = File::create("extremes_eg_2.json")?;
    serde_json::to_writer(file, &qube.to_arena_json())?;

    Ok(())
}
