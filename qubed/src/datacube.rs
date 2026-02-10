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
}

impl Qube {
    pub fn to_datacubes(&self) -> Vec<Datacube> {
        let mut datacubes = Vec::new();

        let datacube_paths = self.leaf_node_ids_paths();
        for datacube_path in datacube_paths {
            let mut datacube = Datacube::new();
            for node_id in datacube_path {
                // let actual_node = self.node_ref(node_id).unwrap();
                if let Some(dim) = self.dimension_str(self.node_dim(node_id).unwrap()) {
                    if let Some(coords) =
                        // self.node(node_id).and_then(|node| Some(node.coordinates()))
                        self.node(node_id).map(|node| node.coordinates().clone())
                    {
                        datacube.add_coordinate(&dim, coords.clone());
                    }
                }
            }
            datacubes.push(datacube);
        }
        // for child_id in self.get_span_of(self.root()).unwrap_or_default() {
        //     let mut datacube = Datacube::new();
        //     if let Some(dim) = self.dimension_str(*child_id) {
        //         if let Some(coords) = self.node(*child_id).and_then(|node| Some(node.coordinates())) {
        //             datacube.add_coordinate(&dim, coords.clone());
        //         }
        //     }
        //     datacubes.push(datacube);
        // }

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
                        .create_child(&dim, parent, Some(coords.clone()))
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
                .create_child(&dim, parent, Some(coords.clone()))
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
        // TODO: implement this function
        let mut other_qube = Self::from_datacube(&_datacube, _order);
        self.union(&mut other_qube);
        // unimplemented!();
        // Easier to construct a Qube and then merge. Need to implement merge.
        // todo!()

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
        //             parent = self.create_child(&dim, parent, Some(coords)).expect("Failed to create dimension");
        //             used_dimensions.push(dim);
        //         }
        //     } else {
        //         break; // No more dimensions to process
        //     }
        // }
    }
}
