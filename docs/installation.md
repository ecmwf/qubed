# Installation

To install the latest stable release from PyPI (recommended):

```bash
pip install qubed
```

Or to build and install the latest version from github (requires rust):

```bash
pip install qubed@git+https://github.com/ecmwf/qubed.git@main
```

## Development

To build the develop branch from source install a rust toolchain and pip install maturin then run:

```
git clone -b develop git@github.com:ecmwf/qubed.git
cd qubed
maturin develop
```
