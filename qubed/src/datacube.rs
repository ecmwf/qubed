use crate::{Coordinates, NodeIdx, Qube};
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

    /// Partition the Qube into sub-Qubes grouped by the resolved values of the
    /// given metadata `key`.
    ///
    /// For each leaf path the deepest node that carries `key` directly is located.
    /// Its coordinates are then paired with the metadata values in sorted order
    /// (matching the convention used by `PerCoordStrings` and `Strings`):
    ///
    /// - **Single value** — the whole leaf path goes into that one bucket unchanged.
    /// - **N values, N enumerable coordinates** — the node is split: each single
    ///   coordinate is paired with the corresponding value and added to that
    ///   bucket's path separately.
    /// - **Mismatch / non-enumerable** — the whole leaf path is added to every
    ///   value bucket (conservative fallback).
    ///
    /// Leaves with no value for `key` are not included in any bucket.
    /// Each returned sub-Qube is compressed before being returned.
    pub fn partition_by_metadata(&self, key: &str) -> HashMap<String, Qube> {
        let mut buckets: HashMap<String, Qube> = HashMap::new();

        for path in self.leaf_node_ids_paths() {
            // Walk root → leaf; the deepest node that carries `key` directly
            // is the effective metadata node (child wins over ancestor).
            let mut meta_nid: Option<NodeIdx> = None;
            for &nid in &path {
                if let Some(m) = self.get_node_metadata(nid) {
                    if m.get(key).is_some() {
                        meta_nid = Some(nid);
                    }
                }
            }
            let Some(meta_nid) = meta_nid else { continue };

            let values = self
                .get_node_metadata(meta_nid)
                .and_then(|m| m.get(key))
                .expect("metadata key must exist on meta_nid");

            let raw_coords = self
                .node(meta_nid)
                .map(|n| n.coordinates().clone())
                .expect("meta_nid must be a valid node");

            // If the metadata was consolidated to an ancestor with Empty coords
            // (e.g. the virtual root after try_consolidate_metadata ran), find the
            // first real node in the path below meta_nid that has non-empty coords.
            // That node becomes the effective split point; the metadata values come
            // from meta_nid as normal.
            let (effective_nid, coords) = if !raw_coords.is_empty() {
                (meta_nid, raw_coords)
            } else {
                let maybe = path
                    .iter()
                    .skip_while(|&&n| n != meta_nid)
                    .skip(1) // skip meta_nid itself
                    .find(|&&n| {
                        self.node(n).map(|node| !node.coordinates().is_empty()).unwrap_or(false)
                    })
                    .copied();
                match maybe {
                    Some(nid) => {
                        let c = self.node(nid).map(|n| n.coordinates().clone()).unwrap_or_default();
                        (nid, c)
                    }
                    None => continue, // no real partition point — skip this path
                }
            };

            // Build (single_coordinate, bucket_value) pairs.
            let pairs = pair_coords_with_metadata(&coords, values);

            for (single_coord, value_str) in pairs {
                let bucket = buckets.entry(value_str).or_insert_with(Qube::new);

                // Build a single-path Qube where effective_nid carries only
                // `single_coord`; all other path nodes keep their full coords.
                let mut path_qube = Qube::new();
                let mut parent = path_qube.root();
                for &nid in &path {
                    // Skip the virtual root node – it has no real dimension and
                    // its Coordinates::Empty would be pruned by compress(), taking
                    // all real dimension children with it.
                    if nid == self.root() {
                        continue;
                    }
                    let Some(dim_str) = self.node_dim(nid).and_then(|d| self.dimension_str(d))
                    else {
                        continue;
                    };
                    let node_coords = if nid == effective_nid {
                        single_coord.clone()
                    } else {
                        match self.node(nid).map(|n| n.coordinates().clone()) {
                            Some(c) => c,
                            None => continue,
                        }
                    };
                    parent = path_qube
                        .get_or_create_child(dim_str, parent, Some(node_coords))
                        .expect("partition_by_metadata: failed to create node");
                }
                bucket.append(&mut path_qube);
            }
        }

        for bucket in buckets.values_mut() {
            bucket.compress();
        }

        buckets
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

/// Pair each individual coordinate value of `coords` with the corresponding
/// metadata value(s) from `values`, following the sorted-order convention of
/// `PerCoordStrings` and `Strings`.
///
/// Returns a `Vec<(single_coordinate, value_string)>`, with one entry per
/// (coordinate, value) combination:
///
/// - **`PerCoordStrings`** → one pair per (coordinate, inner_string); a coordinate
///   with multiple inner strings produces multiple pairs.
/// - **Single metadata value** → `[(full coords, value)]`.
/// - **N values == N enumerable coordinates** → one pair per coordinate.
/// - **Mismatch / non-enumerable** → `(full coords, value)` for every value.
fn pair_coords_with_metadata(
    coords: &crate::Coordinates,
    values: &crate::metadata::MetadataValues,
) -> Vec<(crate::Coordinates, String)> {
    // PerCoordStrings: each coord slot has its own set of strings.
    if let crate::metadata::MetadataValues::PerCoordStrings(per_coord) = values {
        let singles = coords.split_into_singles();
        if singles.len() == per_coord.len() {
            let mut result = Vec::new();
            for (coord, inner) in singles.into_iter().zip(per_coord.iter()) {
                for val in inner {
                    result.push((coord.clone(), val.clone()));
                }
            }
            return result;
        }
        // Length mismatch fallback: all unique values paired with full coords.
        let all_vals = values.as_string_vec();
        return all_vals.into_iter().map(|v| (coords.clone(), v)).collect();
    }

    let value_strings = values.as_string_vec();

    if value_strings.len() == 1 {
        return vec![(coords.clone(), value_strings.into_iter().next().unwrap())];
    }

    let singles = coords.split_into_singles();
    if singles.len() == value_strings.len() {
        singles.into_iter().zip(value_strings.into_iter()).collect()
    } else {
        value_strings.into_iter().map(|v| (coords.clone(), v)).collect()
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
