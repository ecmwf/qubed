![Static Badge](https://img.shields.io/badge/ESEE-Production_Chain-blue?style=flat&label=ESEE&link=github.com%2Fecmwf)
![Static Badge](https://img.shields.io/badge/ESEE-Data_Provision-purple?style=flat&label=ESEE&link=github.com%2Fecmwf)
![Static Badge](https://img.shields.io/badge/ESEE-User_Interaction-green?style=flat&label=ESEE&link=github.com%2Fecmwf)
![Static Badge](https://img.shields.io/badge/ESEE-Foundation-orange?style=flat&label=ESEE&link=github.com%2Fecmwf)


# Q<sup>3</sup> Quick Querying of Qubes

This repostitory contains a collection of components designed to deliver user friendly cataloging for ecmwf's data. The STAC Server, Frontend and a periodic job to do tree compression can be deployed together to kubernetes using the [helm chart](./helm_chart). Thise deployment can then be accessed either via the Query Builder Web interface or the python client.

## 📦 Components Overview

### 🌲 [Tree Compressor](./tree_compresser)
> **Python/Rust Package**

📋 Lists the datasets in an **FDB** and converts the output into a **compressed tree representation** for fast querying.

---

### 🚀 [STAC Server](./stac_server)
> **FastAPI STAC Server Backend**

- 🌟 Implements our proposed [Datacube STAC Extension](./structured_stac.md).
- 🛠️ Allows efficient traversal of ECMWF's datacubes.
- 🔗 **[Live Example](http://catalogue.lumi.apps.dte.destination-earth.eu/stac?class=d1&dataset=extremes-dt&expver=0001&stream=oper)**.

---

### 🌐 [Query Builder Web](./frontend)
> **Web Frontend**

- 👀 Displays data from the **STAC Server** in an intuitive user interface.
- 🌍 **[Try the Live Demo](http://catalogue.lumi.apps.dte.destination-earth.eu/)**.

---

### TODO: 🐍 [Query Builder Python](./query_builder) 
> **Python Frontend**

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
