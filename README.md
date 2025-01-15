# Q<sup>3</sup> Quick Querying of Qubes

> [!WARNING]
> This project is under development and not yet feature complete or tested.

> [!WARNING]
> This project is BETA and will be experimental for the forseable future. Interfaces and functionality are likely to change, and the project itself may be scrapped. DO NOT use this software in any project/software that is operational.

This repostitory contains a collection of components designed to deliver user friendly cataloging for datacube data. The STAC Server, Frontend and a periodic job to do tree compression can be deployed together to kubernetes using the [helm chart](./helm_chart). Thise deployment can then be accessed either via the Query Builder Web interface or the python client.

## 📦 Components Overview


### 🚀 [Qubed STAC Server](./stac_server)
> **FastAPI STAC Server Backend**

- 🌟 Implements our proposed [Datacube STAC Extension](./structured_stac.md).
- 🛠️ Allows efficient traversal of ECMWF's datacubes.
- Part of the implementation of this is [🌲 Tree Compressor](./tree_compresser), a **compressed tree representation** optimised for storing trees with many duplicated subtress. 
- 🔗 **[Live Example](https://climate-catalogue.lumi.apps.dte.destination-earth.eu/api/stac?root=root&activity=story-nudging%2Cscenariomip&class=d1)**.

---

### 🌐 [Qubed Web Query Builder](./web_query_builder)
> **Web Frontend**

- 👀 Displays data from the **STAC Server** in an intuitive user interface.
- 🌍 **[Try the Live Demo](https://climate-catalogue.lumi.apps.dte.destination-earth.eu/)**.

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
