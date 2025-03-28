# STAC Generalized Datacubes Extension

- **Title:** Generalized Datacubes
- **Identifier:** <https://stac-extensions.github.io/template/v1.0.0/schema.json>
- **Field Name Prefix:** generalized_datacube
- **Scope:** Catalog
- **Extension [Maturity Classification](https://github.com/radiantearth/stac-spec/tree/master/extensions/README.md#extension-maturity):** Proposal
- **Owner**: @TomHodson

This STAC extension borrows the [Draft OGC Records API](https://docs.ogc.org/DRAFTS/20-004.html), specifically the [templated links section](https://docs.ogc.org/DRAFTS/20-004.html#sc_templated_links_with_variables) to give STAC the ability to index very large datasets that conform to a generalised datacube model.

A typical datacube has a fixed set of dimensions `[a, b, c..]` , each of which have a fixed span `{a: ["temp","rainfall"], b : [1-7], c:[True, False]}` such that we can access data by indexing, i.e providing a value for each axis, `a="rainfall", b=1, ...`.  A generalized datacube, by our defintion, allow the dimensions to change during indexing, so choosing `a="rainfall"` might yield a different set of axes from `a="temp"`.

The [STAC Datacube][datacube_extension] extension serves the needs of datacubes that appear in STAC as Items or Collections, i.e as leaves in the tree. This extension instead focussing on allowing STAC to serve as an interface to dynamically explore the branches of generalised datacubes. It does this by adding additional metadata from the OGC Records standard to the children of Catalog entries.

In practice what this proposal does is:

1. Replace `"links":` with `"linkTemplates":` in the Catalog entry following the example of the OGC Records API.
2. To each `rel: Child` object in `linkTemplates`:
    a. Add a `variables` key following the OGC Records API with a list of entries like:
        ```json
"format": {
        "type": "string",
        "enum": [
            "application/vnd.google-earth.kml+xml",
            "application/vnd.google-earth.kmz",
            "image/png",
            "image/jpeg",
            "image/gif",
            "image/png; mode=8bit",
            "application/x-pdf",
            "image/svg+xml",
            "image/tiff"
        ]
        }
        ```
    b. Add a "uriTemplate" key that specifies how to contruct the resulting URL: i.e `http://hostname.tld/app/index.html?class=od&format={}`

This enables a child object to represent a whole axis and its allowed values. Since `href` must now be constructed dynamically, we rempve it and add a `generalized_datacube:href_template` attribute to communicate how to construct the URLs corresponding to particular choice of value or values.

[gen_datacubes]: https://github.com/ecmwf/datacube-spec
[link_objects]: https://github.com/radiantearth/stac-spec/blob/master/commons/links.md#link-object
[datacube_extension]: https://github.com/stac-extensions/datacube

## Examples
A typical `Catalog` entry with this extension:

```json
{
  "type": "Catalog",
  "title": "Operational Data",
  "id": "rainfall",
  "stac_version": "1.0.0",
  "description": "ECMWF's Operational Data Archive",
  "linkTemplates": [
    {
      "rel": "child",
      "title": "Expver - Experiment Version",
      "uriTemplate": "http://hostname.tld/app/index.html?class=od&expver={}",
      "type": "application/json",
      "variables" : {
        "expver" : {
            "description": "Experiment version, 0001 selects operational data.",
            "type" : "string",
            "enum" : ["0001", "xxxx"],
            "value_descriptions" : ["Operational Data", "Experimental Data"],
            "optional" : false,
        }
      }
      ""

    },
  ],
  "stac_extensions": [
    "https://stac-extensions.github.io/generalised_datacubes/v1.0.0/schema.json"
  ],

}
```


## Fields

The fields in the table below can be used in these parts of STAC documents:

- [ ] Catalogs
- [ ] Collections
- [ ] Item Properties (incl. Summaries in Collections)
- [ ] Assets (for both Collections and Items, incl. Item Asset Definitions in Collections)
- [x] Links

| Field Name           | Type                      | Description                                              |
| -------------------- | ------------------------- | -------------------------------------------------------- |
| axis:dimension       | Dimension Object          | Whether the axis is an enum, date range, time range etc  |
| axis:href_template   | string                    | Whether the axis is an enum, date range, time range etc  |




### Additional Field Information

#### axis:dimension



### Dimension Object

The dimension object reuses all those [defined by the datacube extension](https://github.com/stac-extensions/datacube#dimension-object), currently those are Horizontal Spatial Raster, Vertical Spatial, Temporal Dimension, Spatial Vector Dimension, Additional Dimension. They are reproduced below for reference.

These dimension objects are defined in addition:

### Enum Dimension Object


| Field Name       | Type              | Description                                                  |
| ---------------- | ----------------- | ------------------------------------------------------------ |
| type             | string            | **REQUIRED.**  `enum`.                                       |
| description      | string            | Detailed multi-line description to explain the dimension. [CommonMark 0.29](http://commonmark.org/) syntax MAY be used for rich text representation. |
| values           | \[number\|string] | An ordered list of all values, especially useful for [nominal](https://en.wikipedia.org/wiki/Level_of_measurement#Nominal_level) values. |
| value_descriptions           | \[string] | Optionally provide a human readable description for each value.  Useful if the values are codes that have defined meanings. |
| step             | number\|null      | If the dimension consists of [interval](https://en.wikipedia.org/wiki/Level_of_measurement#Interval_scale) values, the space between the values. Use `null` for irregularly spaced steps. |
| unit             | string            | The unit of measurement for the data, preferably compliant to [UDUNITS-2](https://ncics.org/portfolio/other-resources/udunits2/) units (singular). |
| reference_system | string            | The reference system for the data.                           |

An Enum Dimension Object MUST specify `values`.

Dimension objects degined by the datacube extension:

### Horizontal Spatial Raster Dimension Object

A spatial raster dimension in one of the horizontal (x or y) directions.

| Field Name       | Type           | Description                                                  |
| ---------------- | -------------- | ------------------------------------------------------------ |
| type             | string         | **REQUIRED.** Type of the dimension, always `spatial`.       |
| axis             | string         | **REQUIRED.** Axis of the spatial raster dimension (`x`, `y`).      |
| description      | string         | Detailed multi-line description to explain the dimension. [CommonMark 0.29](http://commonmark.org/) syntax MAY be used for rich text representation. |
| extent           | \[number]      | **REQUIRED.** Extent (lower and upper bounds) of the dimension as two-element array. Open intervals with `null` are not allowed. |
| values           | \[number]      | Optionally, an ordered list of all values.                   |
| step             | number\|null   | The space between the values. Use `null` for irregularly spaced steps. |
| reference_system | string\|number\|object | The spatial reference system for the data, specified as [numerical EPSG code](http://www.epsg-registry.org/), [WKT2 (ISO 19162) string](http://docs.opengeospatial.org/is/18-010r7/18-010r7.html) or [PROJJSON object](https://proj.org/specifications/projjson.html). Defaults to EPSG code 4326. |

### Vertical Spatial Dimension Object

A spatial dimension in vertical (z) direction.

| Field Name       | Type             | Description                                                  |
| ---------------- | ---------------- | ------------------------------------------------------------ |
| type             | string           | **REQUIRED.** Type of the dimension, always `spatial`.       |
| axis             | string           | **REQUIRED.** Axis of the spatial dimension, always `z`.     |
| description      | string           | Detailed multi-line description to explain the dimension. [CommonMark 0.29](http://commonmark.org/) syntax MAY be used for rich text representation. |
| extent           | \[number\|null\]   | If the dimension consists of [ordinal](https://en.wikipedia.org/wiki/Level_of_measurement#Ordinal_scale) values, the extent (lower and upper bounds) of the values as two-element array. Use `null` for open intervals. |
| values           | \[number\|string\] | An ordered list of all values, especially useful for [nominal](https://en.wikipedia.org/wiki/Level_of_measurement#Nominal_level) values. |
| step             | number\|null     | If the dimension consists of [interval](https://en.wikipedia.org/wiki/Level_of_measurement#Interval_scale) values, the space between the values. Use `null` for irregularly spaced steps. |
| unit             | string           | The unit of measurement for the data, preferably compliant to [UDUNITS-2](https://ncics.org/portfolio/other-resources/udunits2/) units (singular). |
| reference_system | string\|number\|object | The spatial reference system for the data, specified as [numerical EPSG code](http://www.epsg-registry.org/), [WKT2 (ISO 19162) string](http://docs.opengeospatial.org/is/18-010r7/18-010r7.html) or [PROJJSON object](https://proj.org/specifications/projjson.html). Defaults to EPSG code 4326. |

A Vertical Spatial Dimension Object MUST specify an `extent` or `values`. It MAY specify both.

### Temporal Dimension Object

A temporal dimension based on the ISO 8601 standard. The temporal reference system for the data is expected to be ISO 8601 compliant
(Gregorian calendar / UTC). Data not compliant with ISO 8601 can be represented as an *Additional Dimension Object* with `type` set to `temporal`.

| Field Name | Type            | Description                                                  |
| ---------- | --------------- | ------------------------------------------------------------ |
| type       | string          | **REQUIRED.** Type of the dimension, always `temporal`.      |
| description | string         | Detailed multi-line description to explain the dimension. [CommonMark 0.29](http://commonmark.org/) syntax MAY be used for rich text representation. |
| extent     | \[string\|null] | **REQUIRED.** Extent (lower and upper bounds) of the dimension as two-element array. The dates and/or times must be strings compliant to [ISO 8601](https://en.wikipedia.org/wiki/ISO_8601). `null` is allowed for open date ranges. |
| values     | \[string]       | If the dimension consists of an ordered list of specific values they can be listed here. The dates and/or times must be strings compliant to [ISO 8601](https://en.wikipedia.org/wiki/ISO_8601). |
| step       | string\|null    | The space between the temporal instances as [ISO 8601 duration](https://en.wikipedia.org/wiki/ISO_8601#Durations), e.g. `P1D`. Use `null` for irregularly spaced steps. |

### Spatial Vector Dimension Object

A vector dimension that defines a spatial dimension based on geometries.

| Field Name       | Type           | Description                                                  |
| ---------------- | -------------- | ------------------------------------------------------------ |
| type             | string         | **REQUIRED.** Type of the dimension, always `geometry`.    |
| axes             | \[string]      | Axes of the vector dimension as an ordered set of `x`, `y` and `z`. Defaults to `x` and `y`. |
| description      | string         | Detailed multi-line description to explain the dimension. [CommonMark 0.29](http://commonmark.org/) syntax MAY be used for rich text representation. |
| bbox             | \[number]      | **REQUIRED.** A single bounding box of the geometries as defined for [STAC Collections](https://github.com/radiantearth/stac-spec/blob/master/collection-spec/collection-spec.md#spatial-extent-object), but not nested. |
| values           | \[string\]     | Optionally, a representation of the geometries. This could be a list of WKT strings or other identifiers. |
| geometry_types   | \[[GeoJSON Types](https://www.rfc-editor.org/rfc/rfc7946#section-1.4)] | A set of geometry types. If not present, mixed geometry types must be assumed. |
| reference_system | string\|number\|object | The spatial reference system for the data, specified as [numerical EPSG code](http://www.epsg-registry.org/), [WKT2 (ISO 19162) string](http://docs.opengeospatial.org/is/18-010r7/18-010r7.html) or [PROJJSON object](https://proj.org/specifications/projjson.html). Defaults to EPSG code 4326. |

For a general explanation what a vector datacube and a vector dimension is, please read the article "[Vector Data Cubes](https://r-spatial.org/r/2022/09/12/vdc.html)".

### Additional Dimension Object

An additional dimension that is not `spatial`, but may be `temporal` if the data is not compliant with ISO 8601 (see below).

| Field Name       | Type              | Description                                                  |
| ---------------- | ----------------- | ------------------------------------------------------------ |
| type             | string            | **REQUIRED.** Custom type of the dimension, never `spatial` or `geometry`. |
| description      | string            | Detailed multi-line description to explain the dimension. [CommonMark 0.29](http://commonmark.org/) syntax MAY be used for rich text representation. |
| extent           | \[number\|null]   | If the dimension consists of [ordinal](https://en.wikipedia.org/wiki/Level_of_measurement#Ordinal_scale) values, the extent (lower and upper bounds) of the values as two-element array. Use `null` for open intervals. |
| values           | \[number\|string] | An ordered list of all values, especially useful for [nominal](https://en.wikipedia.org/wiki/Level_of_measurement#Nominal_level) values. |
| step             | number\|null      | If the dimension consists of [interval](https://en.wikipedia.org/wiki/Level_of_measurement#Interval_scale) values, the space between the values. Use `null` for irregularly spaced steps. |
| unit             | string            | The unit of measurement for the data, preferably compliant to [UDUNITS-2](https://ncics.org/portfolio/other-resources/udunits2/) units (singular). |
| reference_system | string            | The reference system for the data.                           |

An Additional Dimension Object MUST specify an `extent` or `values`. It MAY specify both.

Note on "Additional Dimension" with type `temporal`:
You can distinguish the "Temporal Dimension" from an "Additional Dimension" by checking whether the extent exists and contains strings.
So if the `type` equals `temporal` and `extent` is an array of strings/null, then you have a "Temporal Dimension",
otherwise you have an "Additional Dimension".
