use qubed::{Datacube, Qube};
use serde_json::Value;

pub trait FromDssConstraints {
    fn from_dss_constraints(dss_constraints: &Value) -> Result<Qube, String>;
}

impl FromDssConstraints for Qube {
    fn from_dss_constraints(dss_constraints: &Value) -> Result<Qube, String> {
        let qube = Qube::new();

        let datacubes = dss_constraints.as_array().expect("DSS constraints should be a JSON array");

        for datacube in datacubes {
            let qube_part = parse_datacube(datacube);

            let qube_part = match qube_part {
                Ok(dc) => Qube::from_datacube(&dc, None),
                Err(e) => return Err(format!("Failed to parse datacube: {}", e)),
            };

            // add to qube
            print!("Parsed datacube: {}", qube_part.to_ascii());
        }
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
