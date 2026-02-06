use qubed::coordinates::Coordinates;
use qubed::qube::{NodeIdx, Qube};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

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

        // Split the line into dimension and coordinates
        let parts: Vec<&str> = trimmed.split(',').collect();
        let dimension = parts[0].trim();
        let coordinates = if parts.len() > 1 {
            Some(parts[1].split('/').map(|s| s.trim().parse::<i64>().unwrap()).collect())
        } else {
            None
        };

        // Find the parent node at the current depth
        while let Some(&(parent_depth, _)) = parent_stack.last() {
            if parent_depth < depth {
                break;
            }
            parent_stack.pop();
        }

        let parent_id = parent_stack.last().unwrap().1;

        // Create the child node
        let coords = coordinates.map(Coordinates::from);
        let child_id =
            qube.create_child(dimension, parent_id, coords).expect("Failed to create child node");

        // Push the new node onto the stack
        parent_stack.push((depth, child_id));
    }

    Ok(qube)
}

use std::time::Instant;

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/mars.list");

    let start_time = Instant::now();

    let qube = build_qube_from_file(&path).expect("Failed to build Qube from file");

    // let qube = Qube::from_dss_constraints(&dss_json);

    // Stop the timer
    let duration = start_time.elapsed();

    // Print the time taken
    println!("Time taken to construct Qube: {:?}", duration);

    println!("Constructed Qube: {:?}", qube.unwrap().to_ascii());
}
