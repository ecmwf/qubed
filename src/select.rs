
use crate::{Qube, Dimension, QubeNodeId, Coordinates, QubeView};


impl Qube {
    

    // Select takes a dictionary of key-vecvalues pairs and returns a QubeView
    // It does not matter which order the keys are specified

    pub fn select(&self, selection: &std::collections::HashMap<Dimension, Coordinates>) -> Result<QubeView, String> {
        
        let root = self.root();
        let mut view = QubeView::new(self);

        self.select_recurse(selection, root, &mut view)?;







        todo!()
        
    }

    fn select_recurse(&self, selection: &std::collections::HashMap<Dimension, Coordinates>, id: QubeNodeId, view: &mut QubeView) -> Result<(), String> {
        
        let node = self.get_node(id).ok_or(format!("Node {:?} not found", id))?;

        for (child_key, children) in node.children.iter() {

            if selection.contains_key(child_key) {
                // Compute the union of values between child.values and selection[child_key]
                // Going to be easier to introduce a proper QubeNodeValuesMask enum and implement a function to generate it from two QubeNodeValues


                // Need some better helpers for masking children too
                

            }




        }




        Ok(())
    }

    fn foo(&self) {
        // Helper function
    }

}