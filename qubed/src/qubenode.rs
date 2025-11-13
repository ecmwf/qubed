use std::collections::BTreeMap;
use tiny_vec::TinyVec;
use crate::coordinates::Coordinates;
use crate::qube::Dimension;
use crate::qube::QubeNodeId;


#[derive(Debug)]
pub(crate) struct QubeNode {
    dim: Dimension,
    structural_hash: Option<u64>,
    coords: Coordinates,
    parent: Option<QubeNodeId>,
    children: BTreeMap<Dimension, TinyVec<QubeNodeId, 4>>, // maintains order so we can use a mask on it
}


impl QubeNode {

    pub fn new(
        dim: Dimension,
        coords: Coordinates,
        parent: Option<QubeNodeId>,
    ) -> Self {
        QubeNode {
            dim,
            structural_hash: None,
            coords,
            parent,
            children: BTreeMap::new(),
        }
    }

    pub fn dim(&self) -> &Dimension {
        &self.dim
    }

    pub fn coords(&self) -> &Coordinates {
        &self.coords
    }
    pub fn coords_mut(&mut self) -> &mut Coordinates {
        &mut self.coords
    }

    pub fn children(&self) -> &BTreeMap<Dimension, TinyVec<QubeNodeId, 4>> {
        &self.children
    }

    pub fn set_parent(&mut self, parent: Option<QubeNodeId>) {
        self.parent = parent;
        self.structural_hash = None; // Invalidate structural hash
    }

    pub fn add_child(&mut self, dim: Dimension, child_id: QubeNodeId) {
        self.children
            .entry(dim)
            .or_insert_with(TinyVec::new)
            .push(child_id);
        self.structural_hash = None; // Invalidate structural hash
    }

    pub fn parent(&self) -> Option<QubeNodeId> {
        self.parent
    }

    pub fn structural_hash(&self) -> Option<u64> {
        self.structural_hash
    }
    pub(crate) fn set_structural_hash(&mut self, hash: u64) {
        self.structural_hash = Some(hash);
    }
}