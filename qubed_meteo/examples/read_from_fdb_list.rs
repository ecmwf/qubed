use qubed::Qube;
use qubed_meteo::adapters::fdb::FromFDBList;
use rsfdb::{FDB, request::Request};
use serde_json::json;
use std::env;
use std::time::Instant;

fn main() {
    // Create FDB handle
    let config_path =
        env::current_dir().unwrap().join("/Users/male/git/fdb-home/etc/fdb/config.yaml");
    unsafe {
        std::env::set_var("FDB5_CONFIG_FILE", config_path.to_str().expect("Invalid config path"));
    }
    let fdb = FDB::new(None).unwrap();

    let list_request = Request::from_json(json!({
        "class" : "od",
        "expver" : "0001",
        "stream" : "oper",
        "time" : "0000",
        "domain" : "g",
        "levtype" : "sfc",
    }))
    .expect("Failed to create request from JSON");

    // Create a list iterator with splitkey enabled
    let list_iter = fdb.list(&list_request, true, true).expect("Failed to create list iterator");

    println!("FDB list iterator created successfully. Processing entries...");

    let items: Vec<String> = list_iter
        .map(|item| {
            // Start with an empty base string (do not include the uri)
            let mut s = String::new();

            // If splitkey metadata (request) is present, append key=value pairs
            if let Some(metadata) = item.request {
                for kv in metadata.iter() {
                    if !s.is_empty() {
                        s.push(',');
                    }
                    s.push_str(&format!("{}={}", kv.key, kv.value));
                }
            }

            s
        })
        .collect();

    println!("{}", items.join("\n"));

    let start_time = Instant::now();

    let qube = Qube::from_fdb_list(&items).expect("Failed to build Qube from FDB list");

    println!("Qube structure:\n{}", qube.to_ascii());

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
