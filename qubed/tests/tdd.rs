use qubed::Qube;
use qubed::select::SelectMode;

#[test]
fn view() {}

#[test]
fn tdd2() -> Result<(), String> {
    // let mut qube = Qube::new();
    // let class_od = qube.create_child("class", qube.root(), None).unwrap();

    // let class_od_values = qube
    //     .get_coordinates_of_mut(class_od)
    //     .ok_or("No values for class_od".to_string())?;
    // class_od_values.append(1);
    // class_od_values.append(2);
    // class_od_values.append(3);
    // class_od_values.append(4);
    // class_od_values.extend_from_iter([10, 20, 30].into_iter());

    // let _type_fc = qube.create_child("type", class_od, None).unwrap();

    // for child in qube.get_all_children_of(qube.root()).unwrap() {
    //     println!(
    //         "Child of class: {:?}={:?}",
    //         qube.get_dimension_of(*child).unwrap(),
    //         qube.get_coordinates_of(*child).unwrap()
    //     );
    // }

    Ok(())
}

#[test]
fn tdd_select_demo() -> Result<(), String> {
    // Setup: Create a sample Qube using ASCII representation
    let input = r#"root
├── class=1
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=2
    └── expver=0001
        ├── param=1
        ├── param=2
        └── param=3"#;

    let qube = Qube::from_ascii(input)?;

    // Select: class=1 and param=2
    let selection = [("class", &[1]), ("param", &[2])];
    let selected_qube = qube.select(&selection, SelectMode::Default)?;

    // Query and Assert: The selected Qube should only have class=1,
    // with expver=0001 and 0002, each having only param=2
    let selected_root = selected_qube.root();
    let selected_node = selected_qube.node(selected_root).ok_or("Root not found")?;

    // Check that only "class" dimension exists at root
    let child_dims: Vec<_> = selected_node.child_dimensions().collect();
    assert_eq!(child_dims.len(), 1);
    assert_eq!(selected_qube.dimension_str(child_dims[0]), Some("class"));

    // Get the class=1 child
    let class_children: Vec<_> = selected_node.children(*child_dims[0]).unwrap().collect();
    assert_eq!(class_children.len(), 1);

    let class_node = selected_qube.node(class_children[0]).ok_or("Class node not found")?;
    assert_eq!(class_node.coordinates().len(), 1);
    assert!(class_node.coordinates().contains(&1));

    // Under class=1, check expver children
    let expver_dims: Vec<_> = class_node.child_dimensions().collect();
    assert_eq!(expver_dims.len(), 1);
    assert_eq!(selected_qube.dimension_str(expver_dims[0]), Some("expver"));

    let expver_children: Vec<_> = class_node.children(*expver_dims[0]).unwrap().collect();
    assert_eq!(expver_children.len(), 2); // 0001 and 0002

    // Check each expver has only param=2
    for expver_child_id in expver_children {
        let expver_node = selected_qube.node(expver_child_id).ok_or("Expver node not found")?;
        let param_dims: Vec<_> = expver_node.child_dimensions().collect();
        assert_eq!(param_dims.len(), 1);
        assert_eq!(selected_qube.dimension_str(param_dims[0]), Some("param"));

        let param_children: Vec<_> = expver_node.children(*param_dims[0]).unwrap().collect();
        assert_eq!(param_children.len(), 1);

        let param_node = selected_qube.node(param_children[0]).ok_or("Param node not found")?;
        assert_eq!(param_node.coordinates().len(), 1);
        assert!(param_node.coordinates().contains(&2));
    }

    Ok(())
}
