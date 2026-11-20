use ::qubed::Qube;
use ::qubed_meteo::adapters::catalogue::{Catalogue, MarsRequest, Selection};
use ::qubed_meteo::adapters::from_records;
#[cfg(feature = "rsfdb-support")]
use ::qubed_meteo::adapters::fdb::FromFDBList;
use ::qubed_meteo::adapters::mars_list::FromMARSList;
use ::qubed_meteo::adapters::opendata::FromOpenData;
use ::qubed_meteo::adapters::to_constraints::ToDssConstraints;
use ::qubed_meteo::adapters::from_constraints::FromDssConstraints;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3::wrap_pyfunction;

/// Convert a `Qube` into a Python `qubed.Qube` object by delegating to
/// `qubed.Qube.from_arena_json()` at runtime.
///
/// This ensures the returned object is an instance of the canonical `qubed.Qube`
/// Python class (from the `qubed` extension module), rather than a separately
/// compiled copy of `PyQube` baked into `qubed_meteo`.  The two copies are
/// different Python type objects, so `isinstance(obj, qubed.Qube)` would fail
/// without this bridge.
///
/// Arena JSON, not `to_ascii()`'s MARS-list-like text format, is used for the
/// round trip: ascii has no type information (every value looks like bare
/// text), so a string coordinate that happens to look numeric (e.g. a CDS
/// "year": "2025") would silently come back as an integer. Arena JSON keeps
/// the distinction (`"2025"` vs `2025`) exactly.
fn qube_to_py(py: Python<'_>, qube: Qube) -> PyResult<Py<PyAny>> {
    let arena = serde_json::to_string(&qube.to_arena_json())
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let qubed_mod = PyModule::import(py, "qubed")?;
    let qube_class = qubed_mod.getattr("Qube")?;
    Ok(qube_class.call_method1("from_arena_json", (arena,))?.unbind())
}

#[pyfunction]
pub fn from_mars_list_py(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let qube = Qube::from_mars_list(text).map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}

#[pyfunction]
pub fn from_mars_list_file_py(py: Python<'_>, path: &str) -> PyResult<Py<PyAny>> {
    let qube = Qube::from_mars_list_path(std::path::Path::new(path))
        .map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}

/// Crawl the ECMWF open-data catalogue and return the resulting Qube.
///
/// Args:
///     date: Date string in `YYYYMMDD` format (e.g. `"20240315"`).
///     model: Model identifier, e.g. `"ifs"` or `"aifs"`.
///
/// Returns:
///     A `qubed.Qube` object.
#[pyfunction]
pub fn from_opendata_py(py: Python<'_>, date: &str, model: &str) -> PyResult<Py<PyAny>> {
    let qube = Qube::from_opendata(date, model).map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}

// ---------------------------------------------------------------------------
// Catalogue — discovery of meteo catalogues: axes, locations, MARS requests.
// ---------------------------------------------------------------------------

/// Merge a `selection` dict and `**kwargs` into a `Selection`
/// (`{key: [values]}`), the same convention the old Python
/// `omnicat_catalogue_client` used: scalars become one-element lists, and
/// every value is stringified.
fn selection_from_py(
    selection: Option<&Bound<'_, PyDict>>,
    kwargs: Option<&Bound<'_, PyDict>>,
) -> PyResult<Selection> {
    let mut out = Selection::new();
    for dict in [selection, kwargs].into_iter().flatten() {
        for (key, value) in dict.iter() {
            let key: String = key.extract()?;
            let values: Vec<String> = if let Ok(items) = value.try_iter() {
                // Treat Python strings as scalars, not iterables of characters.
                if value.is_instance_of::<pyo3::types::PyString>() {
                    vec![value.str()?.to_string()]
                } else {
                    items.map(|item| Ok::<_, PyErr>(item?.str()?.to_string())).collect::<PyResult<_>>()?
                }
            } else {
                vec![value.str()?.to_string()]
            };
            out.insert(key, values);
        }
    }
    Ok(out)
}

fn mars_request_to_py<'py>(py: Python<'py>, req: &MarsRequest) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    for (k, v) in req {
        dict.set_item(k, v)?;
    }
    Ok(dict.unbind())
}

/// Drives discovery of a meteorological Qube read from a catalogue server:
/// which values remain available on every axis given a selection, which
/// named serving locations have that data, and the maximally-compressed
/// MARS request(s) a selection expands to.
///
/// Build one with `Catalogue.from_catalogue(url, dataset_id)`, or wrap a
/// `qubed.Qube` you already have with `Catalogue.from_qube(qube, dataset_id)`.
#[pyclass(name = "Catalogue", unsendable)]
pub struct PyCatalogue {
    inner: Catalogue,
}

#[pymethods]
impl PyCatalogue {
    /// Wrap an existing `qubed.Qube` (e.g. loaded from a local arena JSON
    /// file) as a `Catalogue`, using the built-in key-ordering tables and no
    /// location/language metadata.
    #[staticmethod]
    pub fn from_qube(qube_ascii: &str, dataset_id: &str) -> PyResult<Self> {
        let qube = Qube::from_ascii(qube_ascii).map_err(PyValueError::new_err)?;
        Ok(PyCatalogue { inner: Catalogue::new(qube, dataset_id) })
    }

    /// Read a `Catalogue` directly from a running Omnicat server's
    /// `/ai-catalogue` API: fetches `/ai-catalogue/config` (for locations,
    /// language, and presentation order) and
    /// `/ai-catalogue/dataset/{dataset_id}/qube` (the dataset's full
    /// hierarchical structure), then performs all further operations
    /// (axes/locations/mars_requests/describe) locally in Rust — no further
    /// network calls are made.
    #[staticmethod]
    pub fn from_catalogue(catalogue_url: &str, dataset_id: &str) -> PyResult<Self> {
        let inner = Catalogue::from_catalogue(catalogue_url, dataset_id)
            .map_err(PyValueError::new_err)?;
        Ok(PyCatalogue { inner })
    }

    /// A human-readable summary: dataset id, datacube count, every axis with
    /// its value count (and description, where available), and configured
    /// serving locations.
    pub fn describe(&self) -> String {
        self.inner.describe()
    }

    /// `{MARS key: [available values]}` after applying `selection`
    /// (a dict, or keyword arguments, or both — e.g.
    /// `cat.axes(stream="clte")` or `cat.axes({"class": "d1"}, stream="clte")`).
    /// Pass nothing for every axis in the whole catalogue.
    #[pyo3(signature = (selection=None, **kwargs))]
    pub fn axes(
        &self,
        py: Python<'_>,
        selection: Option<&Bound<'_, PyDict>>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyDict>> {
        let sel = selection_from_py(selection, kwargs)?;
        let axes = self.inner.axes(&sel).map_err(PyValueError::new_err)?;
        let dict = PyDict::new(py);
        for (k, values) in axes {
            dict.set_item(k, values)?;
        }
        Ok(dict.unbind())
    }

    /// The maximally-compressed MARS request(s) `selection` expands to, as a
    /// list of `{key: "v1/v2/..."}` dicts.
    #[pyo3(signature = (selection=None, **kwargs))]
    pub fn mars_requests(
        &self,
        py: Python<'_>,
        selection: Option<&Bound<'_, PyDict>>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Vec<Py<PyDict>>> {
        let sel = selection_from_py(selection, kwargs)?;
        let reqs = self.inner.mars_requests(&sel).map_err(PyValueError::new_err)?;
        reqs.iter().map(|r| mars_request_to_py(py, r)).collect()
    }

    /// Named serving locations that have data for `selection`, each with its
    /// own independently-compressed MARS requests:
    /// `[{"name", "url", "requests"}, ...]`, sorted by name. `url` is `None`
    /// if the location has no configured data-access URL.
    #[pyo3(signature = (selection=None, **kwargs))]
    pub fn locations(
        &self,
        py: Python<'_>,
        selection: Option<&Bound<'_, PyDict>>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Vec<Py<PyDict>>> {
        let sel = selection_from_py(selection, kwargs)?;
        let locs = self.inner.locations_for(&sel).map_err(PyValueError::new_err)?;
        locs.iter()
            .map(|loc| {
                let dict = PyDict::new(py);
                dict.set_item("name", &loc.name)?;
                dict.set_item("url", loc.url.as_deref())?;
                let reqs: PyResult<Vec<Py<PyDict>>> =
                    loc.requests.iter().map(|r| mars_request_to_py(py, r)).collect();
                dict.set_item("requests", reqs?)?;
                Ok(dict.unbind())
            })
            .collect()
    }

    /// The underlying `qubed.Qube`.
    #[getter]
    pub fn qube(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        qube_to_py(py, self.inner.qube.clone())
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Catalogue(dataset_id={:?}, datacubes={})",
            self.inner.dataset_id,
            self.inner.qube.datacube_count(),
        )
    }
}

#[pymodule]
#[pyo3(name = "qubed_meteo")]
fn py_qubed_meteo_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(from_mars_list_py, m)?)?;
    m.add_function(wrap_pyfunction!(from_mars_list_file_py, m)?)?;
    #[cfg(feature = "rsfdb-support")]
    m.add_function(wrap_pyfunction!(from_fdb_list_py, m)?)?;
    m.add_function(wrap_pyfunction!(to_dss_constraints_py, m)?)?;
    m.add_function(wrap_pyfunction!(from_opendata_py, m)?)?;
    m.add_function(wrap_pyfunction!(from_dss_constraints_py, m)?)?;
    m.add_function(wrap_pyfunction!(from_dense_cubes_py, m)?)?;
    m.add_function(wrap_pyfunction!(from_availability_records_py, m)?)?;
    m.add_class::<PyCatalogue>()?;
    Ok(())
}

#[cfg(feature = "rsfdb-support")]
#[pyfunction]
pub fn from_fdb_list_py(py: Python<'_>, request_json: &str) -> PyResult<Py<PyAny>> {
    let v: serde_json::Value =
        serde_json::from_str(request_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let qube = Qube::from_fdb_list(&v).map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}

#[pyfunction]
pub fn to_dss_constraints_py(ascii: &str) -> PyResult<String> {
    let qube = Qube::from_ascii(ascii).map_err(|e| PyValueError::new_err(e))?;
    let v = qube.to_dss_constraints();
    serde_json::to_string(&v).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction]
pub fn from_dss_constraints_py(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    let value: serde_json::Value =
        serde_json::from_str(text)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let qube = Qube::from_dss_constraints(&value).map_err(|e| PyValueError::new_err(e))?;
    qube_to_py(py, qube)
}

/// Build arena JSON from a union of *dense* cubes (each a `{dim: [values]}`
/// Build a `qubed.Qube` from a union of *dense cubes* (each a `{dim:
/// [values]}` JSON object — e.g. one per CDS/DSS "constraint"). No
/// cross-cube coordinate union is performed: every input object remains its
/// own independent datacube. `preferred_order` controls nesting (and thus
/// compaction) order; unlisted dims sort alphabetically after it.
#[pyfunction]
#[pyo3(signature = (cubes_json, preferred_order=vec![]))]
pub fn from_dense_cubes_py(py: Python<'_>, cubes_json: &str, preferred_order: Vec<String>) -> PyResult<Py<PyAny>> {
    let cubes: Vec<serde_json::Value> =
        serde_json::from_str(cubes_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let qube = from_records::from_dense_cubes(&cubes, &preferred_order).map_err(PyValueError::new_err)?;
    qube_to_py(py, qube)
}

/// Build a `qubed.Qube` from scalar *availability records* (each a flat
/// `{dim: value}` JSON object naming one exact, concrete combination that
/// exists — e.g. one ECMWF open-data `.index` NDJSON line). Unlike
/// `from_dense_cubes_py`, no cartesian product is assumed between records,
/// so availability gaps are preserved exactly. See `from_dense_cubes_py` for
/// the `preferred_order` notes.
#[pyfunction]
#[pyo3(signature = (records_json, preferred_order=vec![]))]
pub fn from_availability_records_py(
    py: Python<'_>,
    records_json: &str,
    preferred_order: Vec<String>,
) -> PyResult<Py<PyAny>> {
    let records: Vec<serde_json::Value> =
        serde_json::from_str(records_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let qube =
        from_records::from_availability_records(&records, &preferred_order).map_err(PyValueError::new_err)?;
    qube_to_py(py, qube)
}
