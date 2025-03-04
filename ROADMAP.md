
Initial Python Implementation
[x] Basic Qube datastructure
[x] Compression
[x] Set Operations (Union, Difference, Intersection...)
[x] Query with request
[x] Iteration over leaves
[x] Iteration over datacubes
[x] Command line creation from fdb list --compact
[ ] Set up periodic updates to climate-dt/extremes-dt again
[ ] Maybe also do production db?
[ ] Do mars list to contraints conversion
[ ] protobuf serialization


Rust port
[ ] Initial object
[ ] Sort out ownership issues, (one arena owned by python object)
[ ] Compression
[ ] Set Operations
[ ] Query with request
[ ] Iteration over leaves
[ ] Iteration over datacubes
[ ] Set up periodic updates to climate-dt/extremes-dt again

## API

Qubed will provide a core compressed tree data structure called a Qube  with:

Methods to convert to and from:
- [x] A human readable representation like those seen above.
- [x] An HTML version where subtrees can be collapsed.
- [ ] An compact protobuf-based binary format
- [x] Nested python dictionaries or JSON
- [/] The output of [fdb list](https://confluence.ecmwf.int/display/FDB/fdb-list)
- [ ] [mars list][mars list]
- [ ] [constraints.json][constraints]

[constraints]: https://object-store.os-api.cci2.ecmwf.int/cci2-prod-catalogue/resources/reanalysis-era5-land/constraints_a0ae5b42d67869674e13fba9fd055640bcffc37c24578be1f465d7d5ab2c7ee5.json
[mars list]: https://git.ecmwf.int/projects/CDS/repos/cads-forms-reanalysis/browse/reanalysis-era5-single-levels/gecko-config/mars.list?at=refs%2Fheads%2Fprod

Useful algorithms:
- [x] Compression
- [/] Union/Intersection/Difference

Performant Membership Queries
- Identifier membership
- Datacube query (selection)

Metadata Storage
