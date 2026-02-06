use qubed::{Datacube, Qube};
use serde_json::Value;

pub trait FromDssConstraints {
    fn from_dss_constraints(dss_constraints: &Value) -> Result<Qube, String>;
}

impl FromDssConstraints for Qube {
    // fn from_dss_constraints(dss_constraints: &Value) -> Result<Qube, String> {
    //     // let mut qube = Qube::new();

    //     let datacubes: &Vec<Value> =
    //         dss_constraints.as_array().expect("DSS constraints should be a JSON array");

    //     let order = vec![
    //         "origin".to_string(),
    //         "forecast_type".to_string(),
    //         "hday".to_string(),
    //         "day".to_string(),
    //         "hmonth".to_string(),
    //         "hyear".to_string(),
    //         "year".to_string(),
    //         "month".to_string(),
    //         "time".to_string(),
    //         "leadtime_hour".to_string(),
    //         "level_type".to_string(),
    //         "variable".to_string(),
    //     ];

    //     let first_datacube = parse_datacube(&datacubes[0])?;
    //     let mut qube = Qube::from_datacube(&first_datacube, Some(&order));
    //     // print!("Partial datacube: {}", qube.to_ascii());

    //     for datacube in &datacubes[1..] {
    //         let qube_part = parse_datacube(datacube);

    //         let mut qube_part = match qube_part {
    //             Ok(dc) => Qube::from_datacube(&dc, Some(&order)),
    //             Err(e) => return Err(format!("Failed to parse datacube: {}", e)),
    //         };

    //         // add to qube
    //         qube.union(&mut qube_part);
    //         // print!("Parsed datacube: {}", qube_part.to_ascii());
    //         // print!("Partial datacube: {}", qube.to_ascii());
    //     }
    //     Ok(qube)
    // }

    fn from_dss_constraints(dss_constraints: &Value) -> Result<Qube, String> {
        let datacubes: &Vec<Value> =
            dss_constraints.as_array().expect("DSS constraints should be a JSON array");

        let order = vec![
            "origin".to_string(),
            "forecast_type".to_string(),
            "hday".to_string(),
            "day".to_string(),
            "hmonth".to_string(),
            "hyear".to_string(),
            "year".to_string(),
            "month".to_string(),
            "time".to_string(),
            "leadtime_hour".to_string(),
            "level_type".to_string(),
            "variable".to_string(),
        ];

        // Parse the first datacube and initialize the main Qube
        let first_datacube = parse_datacube(&datacubes[0])?;
        let mut qube = Qube::from_datacube(&first_datacube, Some(&order));

        // Collect all other Qubes into a Vec<Qube>
        let mut other_qubes: Vec<Qube> = Vec::new();
        for datacube in &datacubes[1..] {
            let qube_part = parse_datacube(datacube);

            let qube_part = match qube_part {
                Ok(dc) => Qube::from_datacube(&dc, Some(&order)),
                Err(e) => return Err(format!("Failed to parse datacube: {}", e)),
            };

            other_qubes.push(qube_part);
        }

        // Use union_many to merge all Qubes together
        qube.union_many(&mut other_qubes);

        Ok(qube)
    }
}

fn parse_datacube(dss_datacube: &Value) -> Result<Datacube, String> {
    let mut datacube = Datacube::new();

    let dimensions = dss_datacube.as_object().expect("DSS datacube should be a JSON object");

    for (dimension_name, coordinates) in dimensions {
        let coord_array = coordinates.as_array().expect(
            format!("Datacube dimension {} should be a JSON array", dimension_name).as_str(),
        );

        let mut coords = qubed::Coordinates::new();

        for coord in coord_array {
            match coord {
                Value::Number(num) => {
                    if num.is_i64() {
                        coords.append(num.as_i64().unwrap() as i32);
                    } else if num.is_f64() {
                        coords.append(num.as_f64().unwrap());
                    }
                }
                Value::String(s) => {
                    coords.append(s.clone());
                }
                _ => panic!("Unsupported coordinate type in dimension {}", dimension_name),
            }
        }

        datacube.add_coordinate(dimension_name, coords);
    }

    Ok(datacube)
}
