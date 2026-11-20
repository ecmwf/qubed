//! Fetches the STAC Collections response from the Copernicus Data Space
//! Sentinel Hub catalog (`GET /catalog/v1/collections`) and ingests it into a
//! `Qube` using the STAC adapter, saving the resulting arena JSON under
//! `examples/data/`.
//!
//! Run with:
//! ```sh
//! cargo run -p qubed_meteo --example ingest_stac_collections --features opendata-support
//! ```

use qubed::Qube;
use qubed_meteo::adapters::stac::FromStacCatalog;
use std::time::Instant;

const SOURCE_URL: &str = "https://sh.dataspace.copernicus.eu/catalog/v1/collections";

fn main() {
    println!("Fetching STAC collections from {SOURCE_URL}...");

    let response = reqwest::blocking::get(SOURCE_URL)
        .unwrap_or_else(|e| panic!("Failed to fetch {SOURCE_URL}: {e}"));

    let status = response.status();
    let body = response.text().expect("Failed to read response body");
    assert!(status.is_success(), "Request failed with status {status}: {body}");

    let stac_json: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse response as JSON");

    let num_collections =
        stac_json.get("collections").and_then(|c| c.as_array()).map(|c| c.len()).unwrap_or(0);
    println!("Fetched {num_collections} collections.");

    let start_time = Instant::now();
    let qube = Qube::from_stac_catalog(&stac_json).expect("Failed to build Qube from STAC catalog");
    println!("Time taken to construct Qube: {:?}", start_time.elapsed());
    println!("Qube has {} leaf nodes.", qube.leaf_node_ids_paths().len());

    let output_path =
        concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/sh_dataspace_stac_collections_qube.json");

    let arena_json = qube.to_arena_json();
    std::fs::write(output_path, serde_json::to_string_pretty(&arena_json).unwrap())
        .expect("Failed to write Qube JSON file");

    println!("Saved arena JSON to {output_path}");
}
