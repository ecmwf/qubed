use crate::{Coordinates, Qube};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Datacube {
    coordinates: HashMap<String, Coordinates>,
}

impl Datacube {
    pub fn new() -> Self {
        Datacube { coordinates: HashMap::new() }
    }

    pub fn add_coordinate(&mut self, dimension: &str, coords: Coordinates) {
        self.coordinates.insert(dimension.to_string(), coords);
    }

    pub fn is_empty(&self) -> bool {
        self.coordinates.is_empty()
    }

    pub fn len(&self) -> usize {
        self.coordinates.len()
    }

    pub fn coordinates(&self) -> &HashMap<String, Coordinates> {
        &self.coordinates
    }
}

impl Qube {
    pub fn to_datacubes(&self) -> Vec<Datacube> {
        let mut datacubes = Vec::new();

        let datacube_paths = self.leaf_node_ids_paths();
        for datacube_path in datacube_paths {
            let mut datacube = Datacube::new();
            for node_id in datacube_path {
                if let Some(dim) = self.dimension_str(self.node_dim(node_id).unwrap()) {
                    if let Some(coords) = self.node(node_id).map(|node| node.coordinates().clone())
                    {
                        datacube.add_coordinate(&dim, coords.clone());
                    }
                }
            }
            datacubes.push(datacube);
        }

        datacubes
    }

    /// Build a Qube from a single Datacube.
    ///
    /// If `order` is provided, dimensions are nested in that order. Any dimensions
    /// not listed in `order` are appended in sorted (alphabetical) order for
    /// deterministic tree structure. When `order` is `None`, all dimensions are
    /// sorted alphabetically.
    pub fn from_datacube(datacube: &Datacube, order: Option<&[String]>) -> Self {
        let mut qube = Qube::new();
        let mut parent = qube.root();

        // Create dimensions in the specified order
        if let Some(order_iter) = order {
            for dim in order_iter {
                if let Some(coords) = datacube.coordinates.get(dim) {
                    parent = qube
                        .get_or_create_child(&dim, parent, Some(coords.clone()))
                        .expect("Failed to create dimension");
                }
            }
        }

        // Create remaining dimensions in sorted order for deterministic tree structure
        let mut remaining: Vec<&String> =
            datacube.coordinates.keys().filter(|dim| qube.dimension(dim).is_none()).collect();
        remaining.sort();

        for dim in remaining {
            let coords = &datacube.coordinates[dim];
            parent = qube
                .get_or_create_child(dim, parent, Some(coords.clone()))
                .expect("Failed to create dimension");
        }

        qube
    }

    pub fn append_datacube(
        &mut self,
        _datacube: Datacube,
        _order: Option<&[String]>,
        _accept_existing_order: bool,
    ) {
        let mut other_qube = Self::from_datacube(&_datacube, _order);
        self.append(&mut other_qube);

        // // we consume the datacube

        // let mut parent = self.root();

        // // If accept_existing_order is true, we try to follow the existing order in the Qube, so check which children exist and use them first
        // // If there are multiple options, choose using the provided order if given, else match first child

        // let mut used_dimensions = vec![];

        // while !datacube.is_empty() {

        //     let mut found = false;

        //     // First try to find existing dimensions in the Qube
        //     for child_dimensions in self.get_span_of(parent).unwrap() {
        //         let dim_name = self.get_dimension_str(child_dimensions).expect("Unknown dimension found");
        //         if let Some(coords) = datacube.coordinates.remove(&dim_name) {
        //             parent = *child_dimensions;
        //             used_dimensions.push(dim_name.clone());
        //             found = true;
        //             break;
        //         }
        //     }

        //     if found {
        //         continue;
        //     }

        //     // If not found, create new dimensions
        //     let next_dim = if let Some(order_iter) = order {
        //         order_iter.iter().find(|d| datacube.coordinates.contains_key(*d)).cloned()
        //     } else {
        //         datacube.coordinates.keys().next().cloned()
        //     };

        //     if let Some(dim) = next_dim {
        //         if let Some(coords) = datacube.coordinates.remove(&dim) {
        //             parent = self.get_or_create_child(&dim, parent, Some(coords)).expect("Failed to create dimension");
        //             used_dimensions.push(dim);
        //         }
        //     } else {
        //         break; // No more dimensions to process
        //     }
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Coordinates;

    fn dc(pairs: &[(&str, &str)]) -> Datacube {
        let mut d = Datacube::new();
        for &(k, v) in pairs {
            d.add_coordinate(k, Coordinates::from_string(v));
        }
        d
    }

    /// Helper to extract the dimension ordering from a Qube's ASCII output.
    /// Returns the dimensions in the order they appear top-to-bottom (root→leaf).
    fn dimension_order(qube: &Qube) -> Vec<String> {
        let ascii = qube.to_ascii();
        let mut dims = Vec::new();
        for line in ascii.lines() {
            if let Some(eq_pos) = line.find('=') {
                // Walk backward from '=' to find the start of the dim name
                let before_eq = &line[..eq_pos];
                let dim_start = before_eq
                    .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let dim = &before_eq[dim_start..];
                if !dim.is_empty() && !dims.contains(&dim.to_string()) {
                    dims.push(dim.to_string());
                }
            }
        }
        dims
    }

    #[test]
    fn from_datacube_with_explicit_order() {
        let datacube = dc(&[("step", "0/6"), ("class", "od"), ("time", "0000")]);
        let order: Vec<String> =
            vec!["class", "time", "step"].into_iter().map(String::from).collect();
        let qube = Qube::from_datacube(&datacube, Some(&order));

        let dims = dimension_order(&qube);
        assert_eq!(dims, vec!["class", "time", "step"]);
    }

    #[test]
    fn from_datacube_no_order_falls_back_to_alphabetical() {
        let datacube = dc(&[("step", "0/6"), ("class", "od"), ("time", "0000")]);
        let qube = Qube::from_datacube(&datacube, None);

        let dims = dimension_order(&qube);
        assert_eq!(dims, vec!["class", "step", "time"]);
    }

    #[test]
    fn from_datacube_partial_order_appends_remaining_alphabetically() {
        let datacube = dc(&[("step", "0/6"), ("class", "od"), ("time", "0000"), ("param", "t")]);
        // Only specify first two dims; step and param should be appended alphabetically
        let order: Vec<String> = vec!["time", "class"].into_iter().map(String::from).collect();
        let qube = Qube::from_datacube(&datacube, Some(&order));

        let dims = dimension_order(&qube);
        assert_eq!(dims, vec!["time", "class", "param", "step"]);
    }

    #[test]
    fn union_preserves_time_split_when_subtrees_differ() {
        // Simulates the ifs-ens case: two datacubes with time=0000/1200 vs time=0600/1800
        // with DIFFERENT step ranges underneath. They must not merge because the subtrees
        // are structurally different.
        let order: Vec<String> = vec!["domain", "time", "type", "stream", "step", "param"]
            .into_iter()
            .map(String::from)
            .collect();

        let dc_a = dc(&[
            ("domain", "g"),
            ("time", "0000/1200"),
            ("type", "fc"),
            ("stream", "oper"),
            ("step", "0/6/12/150/156"),
            ("param", "t/u"),
        ]);
        let dc_b = dc(&[
            ("domain", "g"),
            ("time", "0600/1800"),
            ("type", "fc"),
            ("stream", "oper"),
            ("step", "0/6/12"),
            ("param", "t/u"),
        ]);

        let mut qube_a = Qube::from_datacube(&dc_a, Some(&order));
        let mut qube_b = Qube::from_datacube(&dc_b, Some(&order));
        qube_a.append(&mut qube_b);

        let ascii = qube_a.to_ascii();
        // time=0000/1200 and time=0600/1800 should be separate branches because
        // the step ranges below them differ
        assert!(ascii.contains("time=0000/1200"), "time=0000/1200 branch missing:\n{ascii}");
        assert!(ascii.contains("time=0600/1800"), "time=0600/1800 branch missing:\n{ascii}");
    }

    #[test]
    fn union_merges_time_when_subtrees_identical() {
        // When subtrees below different time values are structurally identical,
        // compress correctly merges them into a single node.
        let order: Vec<String> =
            vec!["domain", "time", "type", "step", "param"].into_iter().map(String::from).collect();

        let dc_a = dc(&[
            ("domain", "g"),
            ("time", "0000/1200"),
            ("type", "fc"),
            ("step", "0/6/12"),
            ("param", "t/u"),
        ]);
        let dc_b = dc(&[
            ("domain", "g"),
            ("time", "0600/1800"),
            ("type", "fc"),
            ("step", "0/6/12"),
            ("param", "t/u"),
        ]);

        let mut qube_a = Qube::from_datacube(&dc_a, Some(&order));
        let mut qube_b = Qube::from_datacube(&dc_b, Some(&order));
        qube_a.append(&mut qube_b);

        let ascii = qube_a.to_ascii();
        // All four times should be merged since subtrees are identical
        assert!(
            ascii.contains("time=0000/0600/1200/1800"),
            "times should be merged when subtrees match:\n{ascii}"
        );
    }

    #[test]
    fn from_datacube_string_coords_not_parsed_as_integers() {
        // "1200" as a string should stay as Strings, not become Integer(1200)
        let mut datacube = Datacube::new();
        let mut coords = Coordinates::new();
        coords.append("1200".to_string());
        datacube.add_coordinate("time", coords);

        let mut coords2 = Coordinates::new();
        coords2.append("0000".to_string());
        coords2.append("1200".to_string());

        // Extending with the same type (Strings) should work without creating Mixed
        coords2.extend(&datacube.coordinates()["time"]);
        assert!(
            !matches!(coords2, Coordinates::Mixed(_)),
            "extending Strings with Strings should not produce Mixed: {:?}",
            coords2
        );
    }
}
