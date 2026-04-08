use qubed::Qube;
use qubed_meteo::adapters::dss_constraints::FromDssConstraints;
use qubed_meteo::adapters::to_constraints::ToDssConstraints;
use std::fs::File;
use std::time::Instant;

fn main() {
    // let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/medium2_era5_constraints.json");
    let path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/constraints_cads_s2s_reforecasts.json");

    let dss_json = std::fs::read_to_string(path).expect("Failed to read DSS constraints JSON file");

    let dss_json =
        serde_json::from_str::<serde_json::Value>(&dss_json).expect("Failed to parse JSON file");

    let start_time = Instant::now();

    let qube = Qube::from_dss_constraints(&dss_json).expect("Failed to build Qube from FDB list");

    // Stop the timer
    let duration = start_time.elapsed();

    // Print the time taken
    println!("Time taken to construct Qube: {:?}", duration);

    println!("Constructed Qube: {:?}", qube.to_ascii());

    let file =
        File::create("s2s_reforecasts_constraints_eg.json").expect("Failed to create JSON file");
    serde_json::to_writer(file, &qube.to_arena_json()).expect("Failed to write Qube to JSON file");

    let start_time_2 = Instant::now();

    let constraints_json = qube.to_dss_constraints();
    // Stop the timer
    let duration_2 = start_time_2.elapsed();
    println!("Time taken to construct datacubes: {:?}", duration_2);

    // Dump the datacubes back to constraints format
    let file = File::create("reconstructed_s2s_reforecasts_constraints_datacubes_eg_large.json")
        .expect("Failed to create JSON file");
    serde_json::to_writer(file, &constraints_json)
        .expect("Failed to write constraints to JSON file");
}
