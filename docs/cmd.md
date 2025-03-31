# Command Line Usage

```bash
fdb list class=rd,expver=0001,... | qubed --from=fdblist --to=text

fdb list --minimum-keys=class class=d1,dataset=climate-dt --config prod_remoteFDB.yaml  | qubed convert --from=fdb --to=text

```

`--from` options include:
* `fdb`

`--to` options include:
* `text`
* `html`
* `json`

use `--input` and `--output` to specify input and output files respectively.


There's some handy test data in the `tests/data` directory. For example:
```bash
gzip -dc tests/data/fdb_list_compact.gz| qubed convert --from=fdb --to=text --output=qube.txt
gzip -dc tests/data/fdb_list_porcelain.gz| qubed convert --from=fdb --to=json --output=qube.json
gzip -dc tests/data/fdb_list_compact.gz | qubed convert --from=fdb --to=html --output=qube.html

// Operational data stream=oper/wave/enfo/waef
fdb list class=od,expver=0001,date=0,stream=oper --compact >> operational_compact.txt
operational_compact.txt | qubed convert --from=fdb --to=text --output=operational.txt
```



## Todo

--from for
* `protobuf`
* `marslist`
* `constraints`

--to for
* `json`
* `datacubes`
* `constraints`
