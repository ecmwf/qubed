use std::collections::BTreeMap;
use tiny_vec::TinyVec;
use crate::Qube;
use crate::coordinates::Coordinates;
use crate::qube::Dimension;
use crate::qube::NodeIdx;


#[derive(Debug)]
pub(crate) struct Node {
    dim: Dimension,
    pub(super) structural_hash: Option<u64>,
    coords: Coordinates,
    pub(super) parent: Option<NodeIdx>,
    children: BTreeMap<Dimension, TinyVec<NodeIdx, 4>>, // maintains order so we can use a mask on it
}


impl Node {

    pub fn new(
        dim: Dimension,
        coords: Coordinates,
        parent: Option<NodeIdx>,
    ) -> Self {
        Node {
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

    pub fn children(&self) -> &BTreeMap<Dimension, TinyVec<NodeIdx, 4>> {
        &self.children
    }


    pub fn add_child(&mut self, dim: Dimension, child_id: NodeIdx) {
        self.children
            .entry(dim)
            .or_insert_with(TinyVec::new)
            .push(child_id);
        self.structural_hash = None; // Invalidate structural hash
    }

    pub fn parent(&self) -> Option<NodeIdx> {
        self.parent
    }

    pub fn structural_hash(&self) -> Option<u64> {
        self.structural_hash
    }
    pub(crate) fn set_structural_hash(&mut self, hash: u64) {
        self.structural_hash = Some(hash);
    }

}