# <p align="center"><img src="https://github.com/ecmwf/qubed/blob/main/docs/_static/banner.svg" width="1000"></p> 
[![Static Badge](https://github.com/ecmwf/codex/raw/refs/heads/main/Project%20Maturity/emerging_badge.svg)](https://github.com/ecmwf/codex/raw/refs/heads/main/Project%20Maturity#emerging)
[![Docs](https://readthedocs.org/projects/qubed/badge/?version=latest)](https://qubed.readthedocs.io/en/latest/)
[![PyPi](https://img.shields.io/pypi/v/qubed.svg)](https://pypi.org/project/qubed/)
[![Wheel](https://img.shields.io/pypi/wheel/qubed.svg)](https://pypi.org/project/qubed/)

Qubed provides a datastructure primitive for working with trees of DataCubes. If a normal tree looks like this:
```
root
├── class=od
│   ├── expver=0001
│   │   ├── param=1
│   │   └── param=2
│   └── expver=0002
│       ├── param=1
│       └── param=2
└── class=rd
    ├── expver=0001
    │   ├── param=1
    │   ├── param=2
    │   └── param=3
    └── expver=0002
        ├── param=1
        └── param=2
```

A compressed view of the same set would be:
```
root
├── class=od, expver=0001/0002, param=1/2
└── class=rd
    ├── expver=0001, param=1/2/3
    └── expver=0002, param=1/2
```

Qubed provides all the algorithms on this data structure you would expect such as intersection/union/difference, compression, search, filtering etc.

In addition to this core datastructure, this repostitory contains a collection of components designed to deliver user friendly cataloging for datacube data. The STAC Server, Frontend and a periodic job to do tree compression can be deployed together to kubernetes using the [helm chart](./helm_chart). Thise deployment can then be accessed either via the Query Builder Web interface or the python client.

## 📦 Components Overview


### 🚀 [Qubed STAC Server](./stac_server)
> **FastAPI STAC Server Backend**

- 🌟 Implements our proposed [Datacube STAC Extension](./structured_stac.md).
- 🛠️ Allows efficient traversal of ECMWF's datacubes.
- Part of the implementation of this is [🌲 Tree Compressor](./tree_compresser), a **compressed tree representation** optimised for storing trees with many duplicated subtress.
- 🔗 **[Live Example](https://qubed.lumi.apps.dte.destination-earth.eu/api/v1/stac/climate-dt/?class=od%2Cd1&dataset=climate-dt)**.

---

### 🌐 [Qubed Web Query Builder](./web_query_builder)
> **Web Frontend**

- 👀 Displays data from the **STAC Server** in an intuitive user interface.
- 🌍 **[Try the Live Demo](https://qubed.lumi.apps.dte.destination-earth.eu/)**.

---

### TODO: 🐍 [Qubed Python Query Builder](./python_query_builder)
> **Python Client**

- 🤖 A Python client for the **STAC Server**.
- 📘 Reference implementation of the [Datacube STAC Extension](./structured_stac.md).

---

## 🚀 Deployment Instructions

Deploy all components to **Kubernetes** using the provided [Helm Chart](./helm_chart).

---

### 🛠️ Future Enhancements
- Intgration **Query Builder Web** with Polytope to contruct a full polytope query.
- A JS polytope client implementation to allow performing the polytope query and getting the result all in the browser.

---
