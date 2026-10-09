use qubed::Qube;
use qubed_meteo::adapters::fdb::{Fdb, FromFDBList};
use std::env;
use std::time::Instant;

fn main() {
    let selector = env::args().nth(1).unwrap_or_else(|| {
        "class=od,expver=0001,stream=oper,time=0000,domain=g,levtype=sfc".into()
    });

    let fdb = Fdb::open_default().expect("open FDB; set FDB5_CONFIG_FILE");
    let start = Instant::now();
    let qube = Qube::from_fdb_list_str(&fdb, &selector).expect("list FDB");

    println!("{}", qube.to_arena_json());
    println!("Time taken to construct Qube: {:?}", start.elapsed());
}
