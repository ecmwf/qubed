use std::collections::HashMap;
use crate::{Coordinates, Qube};

#[derive(Debug)]
pub struct Datacube {
    coordinates: HashMap<String, Coordinates>,
}

impl Datacube {
    pub fn new() -> Self {
        Datacube {
            coordinates: HashMap::new(),
        }
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

    pub fn from_datacube(datacube: &Datacube, order: Option<&[String]>  ) -> Self {
        let mut qube = Qube::new();
        let mut parent = qube.root();
        
        // Create dimensions in the specified order
        if let Some(order_iter) = order {
            for dim in order_iter {
                if let Some(coords) = datacube.coordinates.get(dim) {
                    parent = qube.create_child(&dim, parent, Some(coords.clone())).expect("Failed to create dimension");
                }
            }
        }

        // Create remaining dimensions
        for (dim, coords) in datacube.coordinates.iter() {
            if qube.get_dimension(&dim).is_some() {
                continue;
            }
            parent = qube.create_child(&dim, parent, Some(coords.clone())).expect("Failed to create dimension");
        }

        qube
    }

    pub fn append_datacube(&mut self, _datacube: Datacube, _order: Option<&[String]>, _accept_existing_order: bool) {
        
        unimplemented!();
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