// use qubed::Qube;
// use qubed_meteo::adapters::dss_constraints::FromDssConstraints;
// use std::time::Instant;

// fn main() {
//     let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/medium2_era5_constraints.json");

//     let dss_json = std::fs::read_to_string(path).expect("Failed to read DSS constraints JSON file");

//     let dss_json =
//         serde_json::from_str::<serde_json::Value>(&dss_json).expect("Failed to parse JSON file");

//     let start_time = Instant::now();

//     let qube = Qube::from_dss_constraints(&dss_json);

//     // Stop the timer
//     let duration = start_time.elapsed();

//     // Print the time taken
//     println!("Time taken to construct Qube: {:?}", duration);

//     println!("Constructed Qube: {:?}", qube.unwrap().to_ascii());
// }

// // #[cfg(test)]
// // mod tests {
// //     use super::*;
// //     use serde_json::json;

// //     #[test]
// //     fn test_merge_datacubes() {
// //         // Define the first datacube as JSON
// //         let datacube1 = json!({
// //             "variable": [
// //                 "10m_u_component_of_wind",
// //                 "10m_v_component_of_wind",
// //                 "convective_precipitation",
// //                 "mean_sea_level_pressure",
// //                 "surface_pressure"
// //             ],
// //             "day": ["01"],
// //             "year": ["2017", "2018", "2019", "2020"],
// //             "month": ["01"],
// //             "leadtime_hour": [
// //                 "1008", "1032", "1056", "1080", "1104", "1128", "1152", "1176", "120", "1200",
// //                 "1224", "1248", "1272", "1296", "1320", "1344", "1368", "1392", "1416", "144",
// //                 "1440", "168", "192", "216", "24", "240", "264", "288", "312", "336", "360",
// //                 "384", "408", "432", "456", "48", "480", "504", "528", "552", "576", "600",
// //                 "624", "648", "672", "696", "72", "720", "744", "768", "792", "816", "840",
// //                 "864", "888", "912", "936", "96", "960", "984"
// //             ],
// //             "forecast_type": ["control_forecast", "perturbed_forecast"],
// //             "hday": ["01"],
// //             "hmonth": ["01"],
// //             "hyear": [
// //                 "1991", "1992", "1993", "1994", "1995", "1996", "1997", "1998", "1999", "2000",
// //                 "2001", "2002", "2003", "2004", "2005", "2006", "2007", "2008", "2009", "2010"
// //             ],
// //             "level_type": ["single_level"],
// //             "origin": ["kma"],
// //             "time": ["00:00"]
// //         });

// //         // Define the second datacube as JSON
// //         let datacube2 = json!({
// //             "origin": ["kma"],
// //             "hday": ["01"],
// //             "forecast_type": ["control_forecast", "perturbed_forecast"],
// //             "leadtime_hour": [
// //                 "0_24", "1008_1032", "1032_1056", "1056_1080", "1080_1104", "1104_1128",
// //                 "1128_1152", "1152_1176", "1176_1200", "1200_1224", "120_144", "1224_1248",
// //                 "1248_1272", "1272_1296", "1296_1320", "1320_1344", "1344_1368", "1368_1392",
// //                 "1392_1416", "1416_1440", "144_168", "168_192", "192_216", "216_240", "240_264",
// //                 "24_48", "264_288", "288_312", "312_336", "336_360", "360_384", "384_408",
// //                 "408_432", "432_456", "456_480", "480_504", "48_72", "504_528", "528_552",
// //                 "552_576", "576_600", "600_624", "624_648", "648_672", "672_696", "696_720",
// //                 "720_744", "72_96", "744_768", "768_792", "792_816", "816_840", "840_864",
// //                 "864_888", "888_912", "912_936", "936_960", "960_984", "96_120", "984_1008"
// //             ],
// //             "hmonth": ["01"],
// //             "variable": [
// //                 "2m_dewpoint_temperature",
// //                 "2m_temperature",
// //                 "sea_ice_area_fraction",
// //                 "skin_temperature"
// //             ],
// //             "day": ["01"],
// //             "hyear": [
// //                 "1991", "1992", "1993", "1994", "1995", "1996", "1997", "1998", "1999", "2000",
// //                 "2001", "2002", "2003", "2004", "2005", "2006", "2007", "2008", "2009", "2010"
// //             ],
// //             "level_type": ["single_level"],
// //             "month": ["01"],
// //             "time": ["00:00"]
// //         });

// //         // Combine the two datacubes into a JSON array
// //         let dss_constraints = json!([datacube1, datacube2]);

// //         // Call the `from_dss_constraints` method
// //         let result = Qube::from_dss_constraints(&dss_constraints);

// //         // Assert that the result is Ok
// //         assert!(result.is_ok());

// //         // Get the resulting Qube
// //         let qube = result.unwrap();

// //         // Print the resulting Qube for debugging
// //         println!("Merged Qube: {:?}", qube.to_ascii());

// //         // Add assertions to verify the merged structure
// //         // For example, check that certain dimensions exist in the merged Qube
// //         assert!(qube.contains_dimension("variable"));
// //         assert!(qube.contains_dimension("leadtime_hour"));
// //         assert!(qube.contains_dimension("origin"));
// //     }
// // }

use qubed::Coordinates;
use qubed::{NodeIdx, Qube};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

// fn build_qube_from_file(file_path: &str) -> io::Result<Qube> {
//     let path = Path::new(file_path);
//     let file = File::open(&path)?;
//     let reader = io::BufReader::new(file);

//     let mut qube = Qube::new();
//     let root = qube.root();

//     // Stack to track parent nodes at each depth level
//     let mut parent_stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

//     for line in reader.lines() {
//         let line = line?;
//         let trimmed = line.trim_start();
//         let depth = line.len() - trimmed.len(); // Indentation depth

//         // Split the line into dimension and coordinates
//         let parts: Vec<&str> = trimmed.split(',').collect();
//         let dimension = parts[0].trim();
//         let coordinates = if parts.len() > 1 {
//             Some(parts[1].split('/').map(|s| s.trim().parse::<i64>().unwrap()).collect::<Vec<_>>())
//         } else {
//             None
//         };

//         // Find the parent node at the current depth
//         while let Some(&(parent_depth, _)) = parent_stack.last() {
//             if parent_depth < depth {
//                 break;
//             }
//             parent_stack.pop();
//         }

//         let parent_id = parent_stack.last().unwrap().1;

//         // Create the child node
//         let coords = coordinates.map(Coordinates::from);
//         let child_id = qube.create_child(dimension, parent_id, coords)
//             .expect("Failed to create child node");

//         // Push the new node onto the stack
//         parent_stack.push((depth, child_id));
//     }

//     Ok(qube)
// }

use qubed::IntegerCoordinates;

// fn build_qube_from_file(file_path: &str) -> io::Result<Qube> {
//     let path = Path::new(file_path);
//     let file = File::open(&path)?;
//     let reader = io::BufReader::new(file);

//     let mut qube = Qube::new();
//     let root = qube.root();

//     // Stack to track parent nodes at each depth level
//     let mut parent_stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

//     for line in reader.lines() {
//         let line = line?;
//         let trimmed = line.trim_start();
//         let depth = line.len() - trimmed.len(); // Indentation depth

//         // Split the line into dimension and coordinates
//         let parts: Vec<&str> = trimmed.split(',').collect();
//         let dimension = parts[0].trim();
//         let coordinates = if parts.len() > 1 {
//             Some(
//                 // IntegerCoordinates::from(
//                 //     parts[1]
//                 //         .split('/')
//                 //         .map(|s| s.trim().parse::<i32>().unwrap())
//                 //         .collect::<Vec<_>>(),
//                 // )
//                 IntegerCoordinates::new(
//                     parts[1]
//                         .split('/')
//                         .map(|s| s.trim().parse::<i32>().unwrap())
//                         .collect::<Vec<_>>(),
//                 )
//             )
//         } else {
//             None
//         };

//         // Find the parent node at the current depth
//         while let Some(&(parent_depth, _)) = parent_stack.last() {
//             if parent_depth < depth {
//                 break;
//             }
//             parent_stack.pop();
//         }

//         let parent_id = parent_stack.last().unwrap().1;

//         // Create the child node
//         let coords = coordinates.map(Coordinates::Integers);
//         let child_id = qube
//             .create_child(dimension, parent_id, coords)
//             .expect("Failed to create child node");

//         // Push the new node onto the stack
//         parent_stack.push((depth, child_id));
//     }

//     Ok(qube)
// }

// fn build_qube_from_file(file_path: &str) -> io::Result<Qube> {
//     let path = Path::new(file_path);
//     let file = File::open(&path)?;
//     let reader = io::BufReader::new(file);

//     let mut qube = Qube::new();
//     let root = qube.root();

//     // Stack to track parent nodes at each depth level
//     let mut parent_stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

//     for line in reader.lines() {
//         let line = line?;
//         let trimmed = line.trim_start();
//         let depth = line.len() - trimmed.len(); // Indentation depth

//         // Split the line into dimension and coordinates
//         let parts: Vec<&str> = trimmed.split(',').collect();
//         let dimension = parts[0].trim();
//         let coordinates = if parts.len() > 1 {
//             Some(
//                 Coordinates::from(
//                     parts[1]
//                         .split('/')
//                         .map(|s| s.trim().parse::<i32>().unwrap())
//                         .collect::<Vec<_>>()
//                         .as_slice(), // Convert Vec<i32> to &[i32]
//                 )
//             )
//         } else {
//             None
//         };

//         // Find the parent node at the current depth
//         while let Some(&(parent_depth, _)) = parent_stack.last() {
//             if parent_depth < depth {
//                 break;
//             }
//             parent_stack.pop();
//         }

//         let parent_id = parent_stack.last().unwrap().1;

//         // Create the child node
//         let child_id = qube
//             .create_child(dimension, parent_id, coordinates)
//             .expect("Failed to create child node");

//         // Push the new node onto the stack
//         parent_stack.push((depth, child_id));
//     }

//     Ok(qube)
// }

// use std::time::Instant;

// fn main() {
//     let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/mars.list");

//     let start_time = Instant::now();

//     let qube = build_qube_from_file(&path).expect("Failed to build Qube from file");

//     // let qube = Qube::from_dss_constraints(&dss_json);

//     // Stop the timer
//     let duration = start_time.elapsed();

//     // Print the time taken
//     println!("Time taken to construct Qube: {:?}", duration);

//     println!("Constructed Qube: {:?}", qube.to_ascii());
// }

// use std::collections::HashMap;
// // use std::fs::File;
// // use std::io::{self, BufRead};
// // use std::path::Path;

// /// Represents a parsed line from the file
// #[derive(Debug)]
// struct ParsedLine {
//     attributes: HashMap<String, String>,
// }

// impl ParsedLine {
//     /// Parses a single line into a `ParsedLine` struct
//     fn from_line(line: &str) -> Self {
//         let mut attributes = HashMap::new();

//         // Split the line by commas and process each key-value pair
//         for pair in line.split(',') {
//             if let Some((key, value)) = pair.split_once('=') {
//                 attributes.insert(key.trim().to_string(), value.trim().to_string());
//             }
//         }

//         ParsedLine { attributes }
//     }
// }

// fn main() -> io::Result<()> {
//     // Path to the file
//     // let path = "data.txt";
//     let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/mars.list");

//     // Open the file
//     if let Ok(file) = File::open(&path) {
//         let reader = io::BufReader::new(file);

//         // Parse each line
//         for line in reader.lines() {
//             if let Ok(line) = line {
//                 // Skip empty lines
//                 if line.trim().is_empty() {
//                     continue;
//                 }

//                 // Parse the line
//                 let parsed_line = ParsedLine::from_line(&line);

//                 // Print the parsed line for debugging
//                 println!("{:?}", parsed_line);
//             }
//         }
//     } else {
//         eprintln!("Failed to open the file: {}", path);
//     }

//     Ok(())
// }

// // use std::fs::File;
// // use std::io::{self, BufRead};
// // use std::path::Path;
// // use qubed::{Qube, NodeIdx};
// // use qubed::Coordinates;

// /// Builds a Qube from a file, parsing each line into nodes.
// fn build_qube_from_file(file_path: &str) -> io::Result<Qube> {
//     let path = Path::new(file_path);
//     let file = File::open(&path)?;
//     let reader = io::BufReader::new(file);

//     let mut qube = Qube::new();
//     let root = qube.root();

//     // Stack to track parent nodes at each depth level
//     let mut parent_stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

//     for line in reader.lines() {
//         let line = line?;
//         let trimmed = line.trim_start();
//         let depth = line.len() - trimmed.len(); // Indentation depth

//         // Split the line into key-value pairs
//         for pair in trimmed.split(',') {
//             if let Some((key, value)) = pair.split_once('=') {
//                 let key = key.trim();
//                 let coordinates = value
//                     .split('/')
//                     .map(|s| s.trim().parse::<i64>().unwrap())
//                     .collect::<Vec<_>>();

//                 // Find the parent node at the current depth
//                 while let Some(&(parent_depth, _)) = parent_stack.last() {
//                     if parent_depth < depth {
//                         break;
//                     }
//                     parent_stack.pop();
//                 }

//                 let parent_id = parent_stack.last().unwrap().1;

//                 // Create the child node
//                 let coords = Coordinates::from(coordinates);
//                 let child_id = qube
//                     .create_child(key, parent_id, Some(coords))
//                     .expect("Failed to create child node");

//                 // Push the new node onto the stack
//                 parent_stack.push((depth, child_id));
//             }
//         }
//     }

//     Ok(qube)
// }

// fn main() -> io::Result<()> {
//     // Path to the file
//     let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/mars.list");

//     // Build the Qube
//     let qube = build_qube_from_file(&path).expect("Failed to build Qube from file");

//     // Print the constructed Qube for debugging
//     println!("Constructed Qube: {:?}", qube);

//     Ok(())
// }

// use std::fs::File;
// use std::io::{self, BufRead};
// use std::path::Path;
// use qubed::{Qube, NodeIdx};
// use qubed::Coordinates;

/// Builds a Qube from a file, parsing each line into nodes.
fn build_qube_from_file(file_path: &str) -> io::Result<Qube> {
    let path = Path::new(file_path);
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file);

    let mut qube = Qube::new();
    let root = qube.root();

    // Stack to track parent nodes at each depth level
    let mut parent_stack: Vec<(usize, NodeIdx)> = vec![(0, root)];

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim_start();
        let depth = line.len() - trimmed.len(); // Indentation depth

        // Split the line into key-value pairs
        for pair in trimmed.split(',') {
            if let Some((key, value)) = pair.split_once('=') {
                let key = key.trim();
                let coordinates: Vec<&str> = value.split('/').map(|s| s.trim()).collect();

                // Find the parent node at the current depth
                while let Some(&(parent_depth, _)) = parent_stack.last() {
                    if parent_depth < depth {
                        break;
                    }
                    parent_stack.pop();
                }

                let parent_id = parent_stack.last().unwrap().1;

                // Create the child node
                let coords = Coordinates::from(coordinates.as_slice());
                let child_id = qube
                    .create_child(key, parent_id, Some(coords))
                    .expect("Failed to create child node");

                // Push the new node onto the stack
                parent_stack.push((depth, child_id));
            }
        }
    }

    Ok(qube)
}

fn main() -> io::Result<()> {
    // Path to the file
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/mars.list");

    // Build the Qube
    let qube = build_qube_from_file(&path).expect("Failed to build Qube from file");

    // Print the constructed Qube for debugging
    println!("Constructed Qube: {:?}", qube);

    Ok(())
}
