use std::collections::HashMap;

use crate::{
    Dimension,
    qube::{Qube, QubeNodeId},
};

use smallbitvec::SmallBitVec;

pub struct QubeView<'a> {
    qube: &'a Qube,

    /// Mapping from QubeNodeId to QubeViewNode
    masks: HashMap<QubeNodeId, QubeNodeMask>,
}

struct QubeNodeMask {
    /// The ID of the node in the original Qube
    _node_id: QubeNodeId,
    _values_mask: SmallBitVec,
    children_mask: SmallBitVec,
}

impl QubeView<'_> {
    pub fn new(qube: &Qube) -> QubeView {
        let mut view = QubeView {
            qube,
            masks: HashMap::new(),
        };
        // Always add the root node with empty masks
        let root_id = qube.root();
        let mask = view.create_mask(root_id).unwrap();
        view.masks.insert(root_id, mask);
        view
    }

    pub fn add_to_view(
        &mut self,
        node_id: QubeNodeId,
        _values_mask: SmallBitVec,
        _children_mask: SmallBitVec,
    ) -> Result<(), String> {
        let mask = self.create_mask(node_id)?;

        // Check the parent has already been added. We don't allow floating nodes.
        if let Some(node) = self.qube.get_node(node_id) {
            if let Some(parent_id) = node.parent() {
                if !self.masks.contains_key(&parent_id) && parent_id != self.qube.root() {
                    return Err(format!(
                        "Parent node {:?} has not been added to the view yet",
                        parent_id
                    ));
                }
            }
        } else {
            return Err(format!("Node {:?} not found in the original Qube", node_id));
        }

        self.masks.insert(node_id, mask);

        Ok(())
    }

    fn create_mask(&mut self, node_id: QubeNodeId) -> Result<QubeNodeMask, String> {
        let node = self
            .qube
            .get_node(node_id)
            .ok_or(format!("Node {:?} not found in the original Qube", node_id))?;
        let num_values = node.values_count();
        let num_children = node.children_count();

        let mask = QubeNodeMask {
            _node_id: node_id,
            _values_mask: SmallBitVec::from_elem(num_values, false),
            children_mask: SmallBitVec::from_elem(num_children, false),
        };
        Ok(mask)
    }

    fn get_mask(&self, node_id: QubeNodeId) -> Result<&QubeNodeMask, String> {
        self.masks
            .get(&node_id)
            .ok_or(format!("No mask found for node id {:?}", node_id))
    }

    fn get_node(&self, node_id: QubeNodeId) -> Result<&crate::qubenode::QubeNode, String> {
        self.qube
            .get_node(node_id)
            .ok_or(format!("No node found for id {:?}", node_id))
    }

    pub fn get_all_children_of(
        &self,
        node_id: QubeNodeId,
    ) -> Result<impl Iterator<Item = &QubeNodeId> + '_, String> {
        let mask = self.get_mask(node_id)?;
        let node = self.get_node(node_id)?;
        let mut filtered_children = Vec::new();

        // the mask covers the entire BTreeMap<MiniSpur, TinyVec<QubeNodeId, 4>> so we need to iterate over it
        let mut i = 0;
        for (_child_key, children) in node.children().iter() {
            for child_id in children.iter() {
                i += 1;
                if mask.children_mask.get(i).unwrap_or(false) {
                    filtered_children.push(child_id);
                }
            }
        }
        Ok(filtered_children.into_iter())
    }

    pub fn get_children_of(
        &self,
        node_id: QubeNodeId,
        key: Dimension,
    ) -> Result<impl Iterator<Item = &QubeNodeId> + '_, String> {
        let mask = self.get_mask(node_id)?;
        let node = self.get_node(node_id)?;
        let mut filtered_children = Vec::new();

        // the mask covers the entire BTreeMap<MiniSpur, TinyVec<QubeNodeId, 4>> so we need to iterate over it
        let mut i = 0;
        for (child_key, children) in node.children().iter() {
            if key != *child_key {
                i += node.children()[child_key].len();
                continue;
            }
            for child_id in children.iter() {
                i += 1;
                if mask.children_mask.get(i).unwrap_or(false) {
                    filtered_children.push(child_id);
                }
            }
        }
        Ok(filtered_children.into_iter())
    }
}
