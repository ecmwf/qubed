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
        let mut other_qube = Self::from_datacube(&_datacube, _order);
        self.union(&mut other_qube);
    }
}
