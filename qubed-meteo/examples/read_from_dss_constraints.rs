

use qubed::Qube;
use qubed_meteo::adapters::dss_constraints::FromDssConstraints;

fn main() {

    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/data/era5_constraints.json"
    );

    let dss_json = std::fs::read_to_string(path)
        .expect("Failed to read DSS constraints JSON file");

    let dss_json = serde_json::from_str::<serde_json::Value>(&dss_json)
        .expect("Failed to parse JSON file");
    
    let qube = Qube::from_dss_constraints(&dss_json);
    
    println!("Constructed Qube: {:?}", qube);
}