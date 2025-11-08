use std::collections::{BTreeMap, HashMap};

use slotmap::{SlotMap, new_key_type};
use lasso::{MiniSpur, Rodeo};
use tiny_vec::TinyVec;

use crate::values::QubeNodeValues;

new_key_type! {
    pub struct QubeNodeId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QubeKey(MiniSpur);

#[derive(Debug)]
pub(crate) struct QubeNode {
    key: QubeKey,
    pub values: QubeNodeValues,
    pub _parent: Option<QubeNodeId>,
    pub children: BTreeMap<QubeKey, TinyVec<QubeNodeId, 4>>, // maintains order so we can use a mask on it
}

#[derive(Debug)]
pub struct Qube {
    nodes: SlotMap<QubeNodeId, QubeNode>,
    root_id: QubeNodeId,
    key_store: Rodeo<MiniSpur>,
}

impl Qube {
    pub fn new() -> Self {
        let mut key_store = Rodeo::<MiniSpur>::new();
        let mut nodes = SlotMap::with_key();
        let root_id = nodes.insert(QubeNode {
            key: QubeKey(key_store.get_or_intern("root")),
            values: QubeNodeValues::None(()),
            children: BTreeMap::new(),
            _parent: None,
        });
        
        
        Qube { nodes, root_id, key_store: key_store }
    }


    pub fn root(&self) -> QubeNodeId {
        self.root_id
    }

    pub fn create_child(&mut self, key: &str, parent_id: QubeNodeId, values: Option<QubeNodeValues>) -> Result<QubeNodeId, String> {
        
        if self.nodes.get(parent_id).is_none() {
            return Err(format!("Parent node {:?} not found", parent_id));
        }

        let key = QubeKey(self.key_store.get_or_intern(key));

        let node_id = self.nodes.insert(QubeNode {
            key: key,
            values: values.unwrap_or(QubeNodeValues::None(())),
            children: BTreeMap::new(),
            _parent: Some(parent_id),
        });
        
        let parent = self.get_node_mut(parent_id);
        if let Some(parent) = parent {
            parent.children.entry(key).or_insert_with(TinyVec::new).push(node_id);
        }

        Ok(node_id)
    }

    pub fn get_span_of(&self, id: QubeNodeId) -> Option<impl Iterator<Item = &QubeKey> + '_> {
        self.nodes.get(id).map(|node| node.children.keys())
    }

    pub fn get_children_of(&self, id: QubeNodeId, key: QubeKey) -> Result<impl Iterator<Item = &QubeNodeId> + '_, String> {
        let node = self.nodes.get(id).ok_or(format!("Node {:?} not found", id))?;
        Ok(node.children.get(&key).ok_or(format!("No children with key {:?}", key))?.iter())
    }

    pub fn get_all_children_of(&self, id: QubeNodeId) -> Result<impl Iterator<Item = &QubeNodeId> + '_, String> {
        let node = self.nodes.get(id).ok_or(format!("Node {:?} not found", id))?;
        let all_children = node.children.values().flatten();
        Ok(all_children)
    }

    
    pub fn get_key_of(&self, id: QubeNodeId) -> Option<&str> {
        self.nodes.get(id).and_then(|node| self.key_store.try_resolve(&node.key.0))
    }

    pub fn get_values_of(&self, id: QubeNodeId) -> Option<&QubeNodeValues> {
        self.nodes.get(id).map(|node| &node.values)
    }
    pub fn get_values_of_mut(&mut self, id: QubeNodeId) -> Option<&mut QubeNodeValues> {
        self.get_node_mut(id).map(|node| &mut node.values)
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
    
}

impl QubeNode {
    pub fn children_count(&self) -> usize {
        self.children.values().map(|v| v.len()).sum()
    }
    pub fn values_count(&self) -> usize {
        self.values.len()
    }
}