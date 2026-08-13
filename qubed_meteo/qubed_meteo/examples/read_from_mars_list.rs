use qubed::Qube;
use qubed_meteo::adapters::mars_list::FromMARSList;
use std::time::Instant;

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/data/big_mars.list");

    let mars_list = std::fs::read_to_string(path).expect("Failed to read MARS list file");

    let start_time = Instant::now();
    let qube = Qube::from_mars_list(&mars_list).expect("Failed to parse MARS list");
    let duration = start_time.elapsed();
    println!("Time taken to construct Qube: {:?}", duration);

    let start_time2 = Instant::now();
    let datacubes = qube.to_datacubes();
    let duration2 = start_time2.elapsed();
    println!("Time taken to convert Qube to datacubes: {:?}", duration2);
    println!("Number of datacubes: {}", datacubes.len());

    // Total data-point count across all datacubes (product of all coordinate cardinalities).
    let total_points: u64 = datacubes.iter().map(|dc| {
        dc.coordinates().values().map(|c| c.iter_sorted_strings().len() as u64).product::<u64>()
    }).sum();
    println!("Total data points across all datacubes: {total_points}");

    // Debug: print per-dim counts for datacube [3]
    if let Some(dc3) = datacubes.get(3) {
        print!("  DC[3] per-dim counts: ");
        for (k, c) in dc3.coordinates() {
            print!("{k}={} ", c.iter_sorted_strings().len());
        }
        println!();
    }

    // Print a summary of each datacube to verify correctness.
    for (i, dc) in datacubes.iter().enumerate() {
        let coords = dc.coordinates();
        let get = |k: &str| {
            coords.get(k).map(|c| {
                let vals = c.iter_sorted_strings();
                if vals.len() <= 3 {
                    vals.join("/")
                } else {
                    format!("{}..{} ({} values)", vals[0], vals[vals.len()-1], vals.len())
                }
            }).unwrap_or_else(|| "-".into())
        };
        let point_count: u64 = coords.values().map(|c| c.iter_sorted_strings().len() as u64).product();
        println!(
            "  [{i:>2}] ({point_count:>15} pts) class={} dataset={} levtype={} time={} date={} param={} number={}",
            get("class"), get("dataset"), get("levtype"),
            get("time"), get("date"), get("param"), get("number"),
        );
    }

    // Spot-check: verify a specific flat record from Feb 1940 is reachable.
    // The file contains: class=ea,dataset=mean,levtype=sfc,time=00:00:00/.../21:00:00,date=1940-02-01/...,param=.../9
    // We pick one value from each dimension and check it appears in some datacube.
    let checks: &[(&str, &str)] = &[
        ("class", "ea"),
        ("dataset", "mean"),
        ("levtype", "sfc"),
        ("date", "1940-02-15"),
        ("param", "129"),
    ];

    let found = datacubes.iter().any(|dc| {
        let coords = dc.coordinates();
        checks.iter().all(|(dim, val)| {
            coords.get(*dim)
                .map(|c| c.iter_sorted_strings().iter().any(|v| v.as_str() == *val))
                .unwrap_or(false)
        })
    });
    println!("\nSpot-check (class=ea, dataset=mean, date=1940-02-15, param=129): {}", if found { "FOUND ✓" } else { "MISSING ✗" });

    // Spot-check members dataset with early date.
    let checks2: &[(&str, &str)] = &[
        ("class", "ea"),
        ("dataset", "members"),
        ("date", "1940-03-10"),
        ("number", "5"),
    ];
    let found2 = datacubes.iter().any(|dc| {
        let coords = dc.coordinates();
        checks2.iter().all(|(dim, val)| {
            coords.get(*dim)
                .map(|c| c.iter_sorted_strings().iter().any(|v| v.as_str() == *val))
                .unwrap_or(false)
        })
    });
    println!("Spot-check (dataset=members, date=1940-03-10, number=5):         {}", if found2 { "FOUND ✓" } else { "MISSING ✗" });
}
