# <p align="center"><img src="https://raw.githubusercontent.com/ecmwf/qubed/refs/heads/main/docs/banner.svg" width="1000"></p>
<p align="center">
<a href="https://github.com/ecmwf/codex/raw/refs/heads/main/Project%20Maturity#emerging">
  <img src="https://github.com/ecmwf/codex/raw/refs/heads/main/Project%20Maturity/emerging_badge.svg" alt="Project Maturity">
</a>
<a href="https://ecmwf.github.io/qubed/">
  <img src="https://img.shields.io/badge/Documentation-GitHub%20Pages-blue" alt="Documentation" />
</a>
<a href="https://pypi.org/project/qubed/"><img src="https://img.shields.io/pypi/v/qubed.svg" alt='PyPi'></a>
</p>

Qubed provides a data structure primitive for working with trees of Datacubes. If a normal tree looks like this:
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

Qubed provides all the algorithms on this data structure you would expect, such as unions, compression, search, filtering etc.

