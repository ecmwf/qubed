# Development


## Installation

To install the latest stable release from PyPI (recommended):

```bash
pip install "qubed[cli,stac_server,docs,dev]"
```
Delete optional feature dependencies as appropriate.


To install the latest version from github (requires a rust tool chain):

```bash
pip install qubed@git+https://github.com/ecmwf/qubed.git@main
```

To build the develop branch from source
1. Install a rust toolchain
2. `pip install maturin` then run:

```
git clone -b develop git@github.com:ecmwf/qubed.git
cd qubed
maturin develop
```

## Pre-commit hooks

The repo comes with a `.pre-commit-config.yaml` that should be used to format the code before commiting. See [the pre-commit docs](https://pre-commit.com/) but the gist is:

```bash
pip install pre-commit
pre-commit install # In the root of this repo
```

## CI

The tests are in `./tests`.  The CI is setup using tox to run the tests in a few different environments, they are currently:

* python 3.13
* python 3.12
* python 3.11
* python 3.12 with numpy version 1.x as opposed to version 2.x which is used by default.


## Large File Storage

This repo uses git Large file storage to store some qubes larger than 100MB. These files are not downloaded by `git clone` by default. In order to download them do, for example:
```bash
git lfs pull --include="tests/example_qubes/climate-dt-metadata_2025.cbor" --exclude=""
```

That these files are not downloaded by default is controlled the contents of `.lfsconfig`
