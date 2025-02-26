# Command Line Usage

```bash
fdb list class=rd,expver=0001,... | qubed --from=fdblist --to=text
```

`--from` options include:
* `fdblist`
* `json`
* `protobuf`
* `marslist`
* `constraints`

`--to` options include:
* `text`
* `html`
* `json`
* `datacubes`
* `constraints`

use `--input` and `--output` to specify input and output files respectively.


There's some handy test data in the `tests/data` directory. For example:
```bash
gzip -dc tests/data/fdb_list_compact.gz| qubed --from=fdblist
```
