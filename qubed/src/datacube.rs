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

        // Create remaining dimensions
        for (dim, coords) in datacube.coordinates.iter() {
            if qube.dimension(&dim).is_some() {
                continue;
            }
            parent = qube
                .get_or_create_child(&dim, parent, Some(coords.clone()))
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
/// metadata value from `values`, following the same sorted-order convention as
/// `PerCoordStrings` and `Strings`.
///
/// Returns a `Vec<(single_coordinate, value_string)>`:
///
/// - **Single metadata value** → `[(full coords, value)]` — no splitting needed.
/// - **N values == N enumerable coordinates** → one pair per coordinate.
/// - **Mismatch or non-enumerable** → `(full coords, value)` for every value
///   (conservative fallback: the whole coordinate range goes to every bucket).
fn pair_coords_with_metadata(
    coords: &crate::Coordinates,
    values: &crate::metadata::MetadataValues,
) -> Vec<(crate::Coordinates, String)> {
    let value_strings = values.as_string_vec();

    if value_strings.len() == 1 {
        // Uniform: whole coordinate range belongs to one bucket.
        return vec![(coords.clone(), value_strings.into_iter().next().unwrap())];
    }

    // Multiple values: attempt coordinate-level splitting.
    let singles = coords.split_into_singles();
    if singles.len() == value_strings.len() {
        // Perfect 1-to-1 pairing in sorted order.
        singles.into_iter().zip(value_strings.into_iter()).collect()
    } else {
        // Mismatch (e.g. non-enumerable RangeSet): add full coords to every bucket.
        value_strings.into_iter().map(|v| (coords.clone(), v)).collect()
    }
}
