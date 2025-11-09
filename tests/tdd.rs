use qubed::Qube;
use tiny_vec::tinyvec;

use qubed::IntegerCoordinates;

#[test]
fn view() {}

#[test]
fn tdd2() -> Result<(), String> {
    let mut qube = Qube::new();
    let class_od = qube.create_child("class", qube.root(), None).unwrap();

    let class_od_values = qube
        .get_coordinates_of_mut(class_od)
        .ok_or("No values for class_od".to_string())?;
    class_od_values.append(1);
    class_od_values.append(2);
    class_od_values.append(3);
    class_od_values.append(4);
    class_od_values.extend_from_iter([10, 20, 30].into_iter());

    let _type_fc = qube.create_child("type", class_od, None).unwrap();

    for child in qube.get_all_children_of(qube.root()).unwrap() {
        println!(
            "Child of class: {:?}={:?}",
            qube.get_dimension_of(*child).unwrap(),
            qube.get_coordinates_of(*child).unwrap()
        );
    }

    Ok(())
}
