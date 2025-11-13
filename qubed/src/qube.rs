use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use lasso::{MiniSpur, Rodeo};
use slotmap::{SlotMap, new_key_type};
use tiny_vec::TinyVec;

use crate::coordinates::Coordinates;
use crate::qubenode::QubeNode;

new_key_type! {
    pub struct QubeNodeId;
}

pub struct _QubeString(MiniSpur);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dimension(MiniSpur);


#[derive(Debug)]
pub struct Qube {
    nodes: SlotMap<QubeNodeId, QubeNode>,
    root_id: QubeNodeId,
    key_store: Rodeo<MiniSpur>,
}

impl Qube {
    pub fn new() -> Self {
        let mut string_store = Rodeo::<MiniSpur>::new();
        let mut nodes = SlotMap::with_key();
        let root_id = nodes.insert(QubeNode::new(
            Dimension(string_store.get_or_intern("root")),
            Coordinates::Empty,
            None,
        ));

        Qube {
            nodes,
            root_id,
            key_store: string_store,
        }
    }

    pub fn root(&self) -> QubeNodeId {
        self.root_id
    }

    pub fn create_child(
        &mut self,
        key: &str,
        parent_id: QubeNodeId,
        coordinates: Option<Coordinates>,
    ) -> Result<QubeNodeId, String> {
        if self.nodes.get(parent_id).is_none() {
            return Err(format!("Parent node {:?} not found", parent_id));
        }

        let key = Dimension(self.key_store.get_or_intern(key));

        let node_id = self.nodes.insert(QubeNode::new(
            key,
            coordinates.unwrap_or(Coordinates::Empty),
            Some(parent_id),
        ));

        let parent = self.get_node_mut(parent_id);
        if let Some(parent) = parent {
            parent.add_child(key, node_id);
        }

        Ok(node_id)
    }

    pub fn get_span_of(&self, id: QubeNodeId) -> Option<impl Iterator<Item = &Dimension> + '_> {
        self.nodes.get(id).map(|node| node.children().keys())
    }

    pub fn get_children_of(
        &self,
        id: QubeNodeId,
        key: Dimension,
    ) -> Result<impl Iterator<Item = &QubeNodeId> + '_, String> {
        let node = self
            .nodes
            .get(id)
            .ok_or(format!("Node {:?} not found", id))?;
        Ok(node
            .children()
            .get(&key)
            .ok_or(format!("No children with key {:?}", key))?
            .iter())
    }

    pub fn get_all_children_of(
        &self,
        id: QubeNodeId,
    ) -> Result<impl Iterator<Item = &QubeNodeId> + '_, String> {
        let node = self
            .nodes
            .get(id)
            .ok_or(format!("Node {:?} not found", id))?;
        let all_children = node.children().values().flatten();
        Ok(all_children)
    }

    pub fn get_dimension_of(&self, id: QubeNodeId) -> Option<&str> {
        self.nodes
            .get(id)
            .and_then(|node| self.key_store.try_resolve(&node.dim().0))
    }

    pub fn get_coordinates_of(&self, id: QubeNodeId) -> Option<&Coordinates> {
        self.nodes.get(id).map(|node| node.coords())
    }
    pub fn get_coordinates_of_mut(&mut self, id: QubeNodeId) -> Option<&mut Coordinates> {
        self.get_node_mut(id).map(|node| node.coords_mut())
    }


    // TODO: better naming of these functions?
    pub fn get_dimension(&self, dim_str: &str) -> Option<Dimension> {
        let dim = self.key_store.get(dim_str);
        dim.map(Dimension)
    }
    
    pub fn get_dimension_str(&self, dim: &Dimension) -> Option<&str> {
        self.key_store.try_resolve(&dim.0)
    }

    // Not sure we really need this...
    // pub fn walk(&self, id: QubeNodeId) -> Result<(impl Iterator<Item = &QubeNodeId> + '_, impl Iterator<Item = &QubeNodeId> + '_), String> {

    //     let node = self.nodes.get(id).ok_or(format!("Node {:?} not found", id))?;

    //     let all_children = node.children.values().flatten();
    //     let branches = all_children.filter(move |&id| {
    //         self.get_node(*id).map_or(false, |n| !n.children.is_empty())
    //     });

    //     let all_children = node.children.values().flatten();
    //     let leaves = all_children.filter(move |&id| {
    //         self.get_node(*id).map_or(false, |n| n.children.is_empty())
    //     });

    //     Ok((branches, leaves))
    // }

    // These functions might be a trap. You can't really do anything directly on a node, because almost everything is interned or arena'd inside the Qube.
    // They might have value if you are doing multiple things and want to avoid the repeated lookup
    // We could return a QubeNodeHandle that has a reference to the Qube and the Node, but we don't want to end up duplicating the whole Qube API there.
    // Keeping them private for now
    pub(crate) fn get_node(&self, id: QubeNodeId) -> Option<&QubeNode> {
        self.nodes.get(id)
    }
    fn get_node_mut(&mut self, id: QubeNodeId) -> Option<&mut QubeNode> {
        self.nodes.get_mut(id)
    }


    pub fn get_structural_hash_of(&self, id: QubeNodeId) -> Option<u64> {
        
        let node = self.get_node(id)?;

        if let Some(hash) = node.structural_hash() {
            return Some(hash);
        }

        // Compute a hash of the node dimension and coordinates
        // It must be deterministic between different Qubes

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let version = 1;
        let dimension_string = self.get_dimension_str(&node.dim())?;

        version.hash(&mut hasher);
        dimension_string.hash(&mut hasher);
        node.coords().hash(&mut hasher);
        for (_, child_ids) in node.children().iter() {
            for child_id in child_ids.iter() {
                let child_hash = self.get_structural_hash_of(*child_id)?;
                child_hash.hash(&mut hasher);
            }
        }
        let hash = hasher.finish();
        Some(hash)
    }


}

impl QubeNode {
    pub fn children_count(&self) -> usize {
        self.children().values().map(|v| v.len()).sum()
    }
    pub fn values_count(&self) -> usize {
        self.coords().len()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        // TODO: need much more tests
        let mut qube = Qube::new();
        let root = qube.root();

        let child1 = qube
            .create_child("dim1", root, Some(1.into()))
            .unwrap();
        let child2 = qube
            .create_child("dim2", root, Some(2.into()))
            .unwrap();

        let hash_root = qube.get_structural_hash_of(root).unwrap();
        let hash_child1 = qube.get_structural_hash_of(child1).unwrap();
        let hash_child2 = qube.get_structural_hash_of(child2).unwrap();

        assert_ne!(hash_root, hash_child1);
        assert_ne!(hash_root, hash_child2);
        assert_ne!(hash_child1, hash_child2);
    }
}