use qubed::Qube;
#[cfg(feature = "rsfdb-support")]
use qubed_meteo::adapters::fdb::FromFDBList;
#[cfg(feature = "rsfdb-support")]
use rsfdb::{FDB, request::Request};
use serde_json::json;
use std::env;
use std::time::Instant;

#[cfg(feature = "rsfdb-support")]
fn main() {
    // Ensure FDB config is set so the internal listing can open the DB
    let config_path =
        env::current_dir().unwrap().join("/Users/male/git/fdb-home/etc/fdb/config.yaml");
    unsafe {
        std::env::set_var("FDB5_CONFIG_FILE", config_path.to_str().expect("Invalid config path"));
    }

    let request_map = json!({
        "class" : "od",
        "expver" : "0001",
        "stream" : "oper",
        "time" : "0000",
        "domain" : "g",
        "levtype" : "sfc",
    });
    let start_time = Instant::now();

    // Build the Qube directly from the request; the adapter will open FDB and list.
    let qube = Qube::from_fdb_list(&request_map).expect("Failed to build Qube from FDB list");

    // println!("Qube structure:\n{}", qube.to_ascii());

    println!("Qube in arena json format:\n{}", qube.to_arena_json());

    // Stop the timer
    let duration = start_time.elapsed();

    // Print the time taken
    println!("Time taken to construct Qube: {:?}", duration);

    // let start_time2 = Instant::now();

    // let datacubes = qube.unwrap().to_datacubes();

    // let duration2 = start_time2.elapsed();

    // println!("Time taken to convert Qube to datacubes: {:?}", duration2);
    // println!("Number of datacubes: {}", datacubes.len());
}
