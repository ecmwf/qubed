from qubed import Qube

qube = Qube.empty()

with open("mars.list.efas.2", "r") as file:
    lines = file.readlines()

for line in lines:
    clean_line = line.strip()
    datacube = {
        k: v.split("/") for k, v in (pair.split("=") for pair in clean_line.split(","))
    }
    datacube.pop("hdateyear")  # Can we do this to get more compression?
    subqube = Qube.from_datacube(datacube)
    qube = qube | subqube

qube = qube.compress()

print(qube)
