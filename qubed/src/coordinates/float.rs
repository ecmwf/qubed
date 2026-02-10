// impl FloatCoordinates {
//     fn extend(&mut self, _new_coords: &FloatCoordinates) {
//         todo!()
//     }
//     fn append(&mut self, _new_coord: f64) {
//         todo!()
//     }

//     fn len(&self) -> usize {
//         match self {
//             FloatCoordinates::List(list) => list.len(),
//         }
//     }
//     pub(crate) fn to_string(&self) -> String {
//         match self {
//             FloatCoordinates::List(list) => {
//                 list.iter().map(|v| v.to_string()).collect::<Vec<String>>().join("/")
//             }
//         }
//     }

//     pub(crate) fn hash(&self, hasher: &mut std::collections::hash_map::DefaultHasher) {
//         "floats".hash(hasher);
//         match self {
//             FloatCoordinates::List(list) => {
//                 for val in list.iter() {
//                     val.to_bits().hash(hasher);
//                 }
//             }
//         }
//     }
// }

// impl Default for FloatCoordinates {
//     fn default() -> Self {
//         FloatCoordinates::List(TinyVec::new())
//     }
// }