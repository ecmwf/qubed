use crate::qube::Dimension;
use crate::{NodeIdx, Qube};
use std::collections::HashMap;
use std::time::Instant;

impl Qube {
    pub fn node_union(&mut self, other: &mut Qube, self_id: NodeIdx, other_id: NodeIdx) -> NodeIdx {
        // Performs a union operation between two nodes in two different Qubes.

        // Group the children of both nodes into groups according to their associated dimensions.
        let self_children = {
            let node = self.node_ref(self_id).unwrap();
            node.children().clone()
        };

        let other_children = {
            let node = other.node_ref(other_id).unwrap();
            node.children().clone()
        };

        // Create a map of dimensions to (self_children, other_children).
        let mut dim_child_map: HashMap<Dimension, (Vec<NodeIdx>, Vec<NodeIdx>)> = HashMap::new();

        for (dim, self_kids) in self_children {
            dim_child_map.entry(dim).or_default().0.extend(self_kids);
        }
        for (dim, other_kids) in other_children {
            dim_child_map.entry(dim).or_default().1.extend(other_kids);
        }

        // For each dimension, perform an internal set operation on the groups.
        let dims: Vec<_> = dim_child_map.keys().copied().collect();

        for dim in dims {
            let (these_kids, those_kids) = {
                let entry = dim_child_map.entry(dim).or_default();
                (&entry.0, &entry.1)
            };

            let _new_children = self.internal_set_operation(other, these_kids, those_kids);
        }

        return self.root();
    }

    // pub fn node_union(&mut self, other: &mut Qube, self_id: NodeIdx, other_id: NodeIdx) -> NodeIdx {
    //     // Perform a union operation between two nodes in two different Qubes.

    //     // Group the children of both nodes by their dimensions.
    //     let mut dim_child_map: HashMap<Dimension, (Vec<NodeIdx>, Vec<NodeIdx>)> = HashMap::new();

    //     {
    //         let self_children = self.node_ref(self_id).unwrap().children();
    //         for (dim, self_kids) in self_children {
    //             dim_child_map.entry(*dim).or_default().0.extend(self_kids.iter().copied());
    //         }

    //         let other_children = other.node_ref(other_id).unwrap().children();
    //         for (dim, other_kids) in other_children {
    //             dim_child_map.entry(*dim).or_default().1.extend(other_kids.iter().copied());
    //         }
    //     }

    //     // Perform an internal set operation for each dimension group.
    //     for (dim, (self_kids, other_kids)) in dim_child_map {
    //         self.internal_set_operation(other, &self_kids, &other_kids);
    //     }

    //     self.root()
    // }

    pub fn internal_set_operation(
        &mut self,
        other: &mut Qube,
        self_ids: &Vec<NodeIdx>,
        other_ids: &Vec<NodeIdx>,
    ) -> Option<Vec<NodeIdx>> {
        // Performs a set operation between two groups of nodes from two Qubes.

        let mut return_vec = Vec::new();

        for node in self_ids {
            for other_node in other_ids {
                let self_coords = self.node_ref(*node).unwrap().coords();
                let other_coords = other.node_ref(*other_node).unwrap().coords();

                let (parent_a, dim_a, parent_b, dim_b) = {
                    let actual_node = self.node_ref(*node).unwrap();
                    let actual_other_node = other.node_ref(*other_node).unwrap();

                    (
                        actual_node.parent().unwrap(),
                        actual_node.dim(),
                        actual_other_node.parent().unwrap(),
                        actual_other_node.dim(),
                    )
                };

                // Perform the shallow operation to get the set of values only in self,
                // those only in other, and those in the intersection.
                let intersection_res = self_coords.intersect(other_coords);
                let actual_intersection = intersection_res.intersection;
                let only_self = intersection_res.only_a;
                let only_other = intersection_res.only_b;

                // println!("HERE WHAT DO WE HAVE?: {:?} ", actual_intersection);

                // println!("WHAT IS ONLY IN B?: {:?}", only_other);

                // If the intersection set is non-empty, create new nodes for the intersection
                // and perform a union on them.
                let dim_str = self.dimension_str(dim_a).unwrap().to_owned();
                let other_dim_str = other.dimension_str(dim_b).unwrap().to_owned();

                if actual_intersection.len() != 0 {
                    let check_new_child_a = self.check_if_new_child(
                        &dim_str,
                        parent_a,
                        Some(actual_intersection.clone()),
                    );
                    let check_new_child_b = other.check_if_new_child(
                        &other_dim_str,
                        parent_b,
                        Some(actual_intersection.clone()),
                    );
                    let new_node_a = self
                        .create_child(&dim_str, parent_a, Some(actual_intersection.clone()))
                        .unwrap();

                    let new_node_b = other
                        .create_child(&other_dim_str, parent_b, Some(actual_intersection))
                        .unwrap();

                    if check_new_child_a.unwrap() {
                        // self.add_same_children(new_node_a, *node);
                        self.copy_branch(*node, new_node_a);
                    }
                    if check_new_child_b.unwrap() {
                        // other.add_same_children(new_node_b, *other_node);
                        other.copy_branch(*other_node, new_node_b);
                    }

                    let _nested_result = self.node_union(other, new_node_a, new_node_b);
                }

                // If there are values only in self, update the coordinates of the current node.
                if only_self.len() != 0 {
                    let actual_node = self.node_mut(*node).unwrap();
                    *actual_node.coords_mut() = only_self;
                }

                // If there are values only in other, create a new node for those values.
                if only_other.len() != 0 {
                    // println!("WHAT IS THE QUBE HERE BEFORE: {:?}", self.to_ascii());

                    // TODO: here, we need to pass the dim_str from other actually...
                    let new_node_only_b = self
                        .create_child(&other_dim_str, parent_a, Some(only_other.clone()))
                        .unwrap();

                    // self.add_same_children(new_node_only_b, *other_node);
                    // if self.check_if_new_child(&dim_str, parent_a, Some(only_other.clone())).unwrap(){
                    //     self.copy_subtree(other, *other_node, new_node_only_b);
                    // }

                    self.copy_subtree(other, *other_node, new_node_only_b);

                    // println!(" HERE WHAT KIND OF NODE DID WE ACTUALLY ADD?? {:?}", self.node(new_node_only_b).unwrap().child_dimensions());

                    // println!("WHAT IS THE QUBE HERE NOW: {:?}", self.to_ascii());
                    let actual_other_node = other.node_mut(*other_node).unwrap();
                    *actual_other_node.coords_mut() = only_other;
                }

                {
                    return_vec.push(*node);
                }
            }
        }

        return Some(return_vec);
    }

    pub fn union(&mut self, other: &mut Qube) {
        // Performs a union operation between two Qubes.
        //
        // This method starts at the root of both Qubes and recursively merges their nodes.
        // After the union, the tree is compressed to remove duplicates and empty nodes.

        let self_root_id = self.root();
        let other_root_id = other.root();
        // println!("WHAT IS THE QUBE HERE BEFORE: {:?}", self.to_ascii());
        self.node_union(other, self_root_id, other_root_id);
        // println!("WHAT IS THE QUBE HERE AFTER: {:?}", self.to_ascii());
        self.compress();
    }

    // pub fn union_many(&mut self, others: &mut Vec<Qube>) {

    //     let start_time = Instant::now();

    //     // for other in others.iter_mut() {
    //     //     let self_root_id = self.root();
    //     //     let other_root_id = other.root();

    //     //     // Perform the union with the current Qube
    //     //     self.node_union(other, self_root_id, other_root_id);
    //     // }

    //     let others_len = others.len();
    //     for (i, other) in others.iter_mut().enumerate() {
    //         let self_root_id = self.root();
    //         let other_root_id = other.root();

    //         // Perform the union with the current Qube
    //         self.node_union(other, self_root_id, other_root_id);

    //         // Print progress update
    //         println!("Union completed for Qube {}/{}", i + 1, others_len);
    //     }

    //     // Stop the timer
    //     let duration = start_time.elapsed();

    //     // Print the time taken
    //     println!("Time taken to union Qubes: {:?}", duration);

    //     let start_time_2 = Instant::now();

    //     // Compress the final result after all unions are complete
    //     self.compress();

    //     let duration_2 = start_time_2.elapsed();

    //     // Print the time taken
    //     println!("Time taken to compress Qube: {:?}", duration_2);
    // }

    pub fn union_many(&mut self, others: &mut Vec<Qube>) {
        let start_time = Instant::now();

        let others_len = others.len();
        for (i, other) in others.iter_mut().enumerate() {
            let self_root_id = self.root();
            let other_root_id = other.root();

            // Perform the union with the current Qube
            self.node_union(other, self_root_id, other_root_id);

            // Print progress update
            println!("Union completed for Qube {}/{}", i + 1, others_len);

            // Compress every 1000th Qube
            if (i + 1) % 500 == 0 {
                println!("Compressing after processing {} Qubes...", i + 1);
                self.compress();
            }
        }

        // Stop the timer
        let duration = start_time.elapsed();

        // Print the time taken
        println!("Time taken to union Qubes: {:?}", duration);

        let start_time_2 = Instant::now();

        // Final compression after all unions are complete
        self.compress();

        let duration_2 = start_time_2.elapsed();

        // Print the time taken
        println!("Time taken to compress Qube: {:?}", duration_2);
    }
}

// impl Qube {
//     pub fn union_many(&mut self, others: &mut Vec<Qube>) {
//         let start_time = Instant::now();

//         // Define the batch size
//         let batch_size = 500;

//         // Process the `others` vector in chunks of `batch_size`
//         let mut batch_results: Vec<Qube> = Vec::new();
//         for (batch_index, chunk) in others.chunks_mut(batch_size).enumerate() {
//             let mut batch_qube = Qube::new(); // Temporary Qube for the current batch

//             let chunk_len = chunk.len();
//             for (i, other) in chunk.iter_mut().enumerate() {
//                 let self_root_id = batch_qube.root();
//                 let other_root_id = other.root();

//                 // Perform the union for the current Qube in the batch
//                 batch_qube.node_union(other, self_root_id, other_root_id);

//                 // Print progress update for the current batch
//                 println!(
//                     "Union completed for Qube {}/{} in batch {}",
//                     i + 1,
//                     chunk_len,
//                     batch_index + 1
//                 );
//             }

//             // Compress the batch result
//             println!("Compressing batch {}...", batch_index + 1);
//             batch_qube.compress();

//             // Store the result of the batch
//             batch_results.push(batch_qube);
//         }

//         // Merge all batch results into the main Qube
//         println!("Merging all batch results...");
//         let batch_results_len = batch_results.len();
//         for (i, mut batch_qube) in batch_results.into_iter().enumerate() {
//             let self_root_id = self.root();
//             let batch_root_id = batch_qube.root();

//             // Perform the union with the main Qube
//             self.node_union(&mut batch_qube, self_root_id, batch_root_id);

//             // Print progress update for merging batches
//             println!("Merged batch {}/{}", i + 1, batch_results_len);
//         }

//         // Stop the timer
//         let duration = start_time.elapsed();

//         // Print the time taken
//         println!("Time taken to union Qubes: {:?}", duration);

//         let start_time_2 = Instant::now();

//         // Final compression after all unions are complete
//         self.compress();

//         let duration_2 = start_time_2.elapsed();

//         // Print the time taken
//         println!("Time taken to compress Qube: {:?}", duration_2);
//     }
// }

// impl Qube {
//     pub fn union_many(&mut self, others: &mut Vec<Qube>) {
//         let start_time = Instant::now();

//         // Define the batch size
//         let batch_size = 500;

//         // Process the `others` vector in chunks of `batch_size`
//         let mut batch_results: Vec<Qube> = Vec::new();
//         for (batch_index, chunk) in others.chunks_mut(batch_size).enumerate() {
//             // Move the first Qube in the batch into `batch_qube`
//             let mut batch_qube = chunk
//                 .get_mut(0)
//                 .expect("Batch should not be empty")
//                 .take(); // Take ownership of the first Qube in the batch

//             let chunk_len = chunk.len();
//             for (i, other) in chunk.iter_mut().enumerate() {
//                 // Skip the first Qube in the batch since it's already the `batch_qube`
//                 if i == 0 {
//                     continue;
//                 }

//                 let self_root_id = batch_qube.root();
//                 let other_root_id = other.root();

//                 // Perform the union for the current Qube in the batch
//                 batch_qube.node_union(other, self_root_id, other_root_id);

//                 // Print progress update for the current batch
//                 println!(
//                     "Union completed for Qube {}/{} in batch {}",
//                     i + 1,
//                     chunk_len,
//                     batch_index + 1
//                 );
//             }

//             // Compress the batch result
//             println!("Compressing batch {}...", batch_index + 1);
//             batch_qube.compress();

//             // Store the result of the batch
//             batch_results.push(batch_qube);
//         }

//         // Merge all batch results into the main Qube
//         println!("Merging all batch results...");
//         let batch_results_len = batch_results.len();
//         for (i, mut batch_qube) in batch_results.into_iter().enumerate() {
//             let self_root_id = self.root();
//             let batch_root_id = batch_qube.root();

//             // Perform the union with the main Qube
//             self.node_union(&mut batch_qube, self_root_id, batch_root_id);

//             // Print progress update for merging batches
//             println!("Merged batch {}/{}", i + 1, batch_results_len);
//         }

//         // Stop the timer
//         let duration = start_time.elapsed();

//         // Print the time taken
//         println!("Time taken to union Qubes: {:?}", duration);

//         let start_time_2 = Instant::now();

//         // Final compression after all unions are complete
//         self.compress();

//         let duration_2 = start_time_2.elapsed();

//         // Print the time taken
//         println!("Time taken to compress Qube: {:?}", duration_2);
//     }
// }
