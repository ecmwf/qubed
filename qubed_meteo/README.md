# qubed-meteo

Qubed-meteo is an adapter layer for Qubed.

Qubed provides the underlying compressed tree data structure for representing sparse collections of datacubes. Qubed-meteo adds meteorological adapters for constructing and exchanging those structures.

For more information about Qubed itself, see the Qubed documentation.


## Features

- Parse indentation-based MARS lists into Qubes.

- Build Qubes by crawling the ECMWF Open Data catalogue.

- Convert Qubes to DSS constraint JSON.

- Build Qubes from DSS constraint JSON.

- Optionally support FDB lists when built with FDB support.