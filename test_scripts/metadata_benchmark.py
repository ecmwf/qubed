import json
import logging
import time

from qubed import Qube


def main():
    logger = logging.getLogger("qubed.set_operations")
    logger.setLevel(logging.INFO)

    paths = {}
    i = 0
    t0 = time.perf_counter()

    qube = Qube.empty()
    with open("./tests/data/climate_dt_paths.json") as f:
        for i, line in enumerate(f.readlines()):
            i += 1
            j = json.loads(line)

            if "type" in j and j["type"] == "path":
                paths[j["i"]] = j["path"]

            else:
                request = j.pop("keys")
                metadata = j
                metadata["path"] = paths[metadata["path"]]

                q = (
                    Qube.from_datacube(request)
                    # .add_metadata(metadata)
                    .add_metadata({"path": metadata["path"]})
                    .add_metadata({"offset": metadata["offset"]}, depth=17)
                    .add_metadata({"length": metadata["length"]}, depth=17)
                )

                qube = qube | q
                if i >= 4000:
                    break

    dt = time.perf_counter() - t0
    per = dt / i * 1000
    per_baseline = 1.33
    print(f"Done in {dt:.2f}s {per:.2f} ms per iteration")
    print(f"{(per_baseline - per) / per_baseline * 100:.2f}% better than baseline")

    print(i)


if __name__ == "__main__":
    main()

    # python -m cProfile -s cumtime test_scripts/metadata_benchmark.py
