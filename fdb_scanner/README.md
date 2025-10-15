The main script here is the scan.py script which uses `fdb axes` and `fdb list` to get a Qube tree that the Stac server uses.

That script is run as a cronjob with the corrent crontab shown below.

Additionally, we suggest running initial scans and using the utility script `kube_init.sh` to upload the files to the pvc where the Stac-server can read it upon restart.

The fdb dumper and dump parser scripts are still in development.
They will be used similarly to scan.py but will also save additional structural metadata to the qube which will later be used by [polytope](https://github.com/ecmwf/polytope).
```sh
# On Demand Extremes DT Full scan every day at 4am
0 4 * * * cd /home/eouser/qubed && ./.venv/bin/python3.12 ./fdb_scanner/scan.py --quiet --full --selector class=d1,dataset=on-demand-extremes-dt --filepath ./fdb_scanner/data/cronjobs/on-demand-extremes-dt.json >> ./fdb_scanner/logs/on-demand-extremes-dt-full-daily.log 2>&1

# On Demand Extremes DT Partial scan every three hours
37 */3 * * * cd /home/eouser/qubed && ./.venv/bin/python3.12 ./fdb_scanner/scan.py --quiet --last_n_days=14 --selector class=d1,dataset=on-demand-extremes-dt --filepath ./fdb_scanner/data/cronjobs/on-demand-extremes-dt.json >> ./fdb_scanner/logs/on-demand-extremes-dt-partial-hourly.log 2>&1

# Extremes-dt Daily Partial scan every three hours
12 */3 * * * cd /home/eouser/qubed && ./.venv/bin/python3.12 ./fdb_scanner/scan.py --quiet --last_n_days=14 --selector class=d1,dataset=extremes-dt --filepath ./fdb_scanner/data/cronjobs/extremes-dt.json >> ./fdb_scanner/logs/extremes-dt.log 2>&1

# Climate dt gen 2 Weekly on sunday at 2am
0 2 * * SUN cd /home/eouser/qubed && ./.venv/bin/python3.12 ./fdb_scanner/scan.py --quiet --full --selector class=d1,dataset=climate-dt,generation=2 --filepath ./fdb_scanner/data/cronjobs/climate-dt-gen-2.json >> ./fdb_scanner/logs/climate-dt.log 2>&1
```