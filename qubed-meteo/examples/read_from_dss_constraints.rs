use qubed::Qube;
use qubed_meteo::adapters::dss_constraints::FromDssConstraints;
use std::time::Instant;

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/era5_constraints.json");

    let dss_json = std::fs::read_to_string(path).expect("Failed to read DSS constraints JSON file");

    let dss_json =
        serde_json::from_str::<serde_json::Value>(&dss_json).expect("Failed to parse JSON file");

    let start_time = Instant::now();

    let qube = Qube::from_dss_constraints(&dss_json);

    // Stop the timer
    let duration = start_time.elapsed();

    // Print the time taken
    println!("Time taken to construct Qube: {:?}", duration);

    // println!("Constructed Qube: {:?}", qube.unwrap().to_ascii());
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use serde_json::json;

//     #[test]
//     fn test_merge_datacubes() {
//         // Define the first datacube as JSON
//         let datacube1 = json!({
//             "variable": [
//                 "10m_u_component_of_wind",
//                 "10m_v_component_of_wind",
//                 "convective_precipitation",
//                 "mean_sea_level_pressure",
//                 "surface_pressure"
//             ],
//             "day": ["01"],
//             "year": ["2017", "2018", "2019", "2020"],
//             "month": ["01"],
//             "leadtime_hour": [
//                 "1008", "1032", "1056", "1080", "1104", "1128", "1152", "1176", "120", "1200",
//                 "1224", "1248", "1272", "1296", "1320", "1344", "1368", "1392", "1416", "144",
//                 "1440", "168", "192", "216", "24", "240", "264", "288", "312", "336", "360",
//                 "384", "408", "432", "456", "48", "480", "504", "528", "552", "576", "600",
//                 "624", "648", "672", "696", "72", "720", "744", "768", "792", "816", "840",
//                 "864", "888", "912", "936", "96", "960", "984"
//             ],
//             "forecast_type": ["control_forecast", "perturbed_forecast"],
//             "hday": ["01"],
//             "hmonth": ["01"],
//             "hyear": [
//                 "1991", "1992", "1993", "1994", "1995", "1996", "1997", "1998", "1999", "2000",
//                 "2001", "2002", "2003", "2004", "2005", "2006", "2007", "2008", "2009", "2010"
//             ],
//             "level_type": ["single_level"],
//             "origin": ["kma"],
//             "time": ["00:00"]
//         });

//         // Define the second datacube as JSON
//         let datacube2 = json!({
//             "origin": ["kma"],
//             "hday": ["01"],
//             "forecast_type": ["control_forecast", "perturbed_forecast"],
//             "leadtime_hour": [
//                 "0_24", "1008_1032", "1032_1056", "1056_1080", "1080_1104", "1104_1128",
//                 "1128_1152", "1152_1176", "1176_1200", "1200_1224", "120_144", "1224_1248",
//                 "1248_1272", "1272_1296", "1296_1320", "1320_1344", "1344_1368", "1368_1392",
//                 "1392_1416", "1416_1440", "144_168", "168_192", "192_216", "216_240", "240_264",
//                 "24_48", "264_288", "288_312", "312_336", "336_360", "360_384", "384_408",
//                 "408_432", "432_456", "456_480", "480_504", "48_72", "504_528", "528_552",
//                 "552_576", "576_600", "600_624", "624_648", "648_672", "672_696", "696_720",
//                 "720_744", "72_96", "744_768", "768_792", "792_816", "816_840", "840_864",
//                 "864_888", "888_912", "912_936", "936_960", "960_984", "96_120", "984_1008"
//             ],
//             "hmonth": ["01"],
//             "variable": [
//                 "2m_dewpoint_temperature",
//                 "2m_temperature",
//                 "sea_ice_area_fraction",
//                 "skin_temperature"
//             ],
//             "day": ["01"],
//             "hyear": [
//                 "1991", "1992", "1993", "1994", "1995", "1996", "1997", "1998", "1999", "2000",
//                 "2001", "2002", "2003", "2004", "2005", "2006", "2007", "2008", "2009", "2010"
//             ],
//             "level_type": ["single_level"],
//             "month": ["01"],
//             "time": ["00:00"]
//         });

//         // Combine the two datacubes into a JSON array
//         let dss_constraints = json!([datacube1, datacube2]);

//         // Call the `from_dss_constraints` method
//         let result = Qube::from_dss_constraints(&dss_constraints);

//         // Assert that the result is Ok
//         assert!(result.is_ok());

//         // Get the resulting Qube
//         let qube = result.unwrap();

//         // Print the resulting Qube for debugging
//         println!("Merged Qube: {:?}", qube.to_ascii());

//         // Add assertions to verify the merged structure
//         // For example, check that certain dimensions exist in the merged Qube
//         assert!(qube.contains_dimension("variable"));
//         assert!(qube.contains_dimension("leadtime_hour"));
//         assert!(qube.contains_dimension("origin"));
//     }
// }
