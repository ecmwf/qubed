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

In practice, what this proposal does is:

1. For child items that represent many distinct children, replace `"links":` with `"linkTemplates":` in the Catalog entry. (Following the example of the OGC Records API.)
2. For each `rel: Child` object in `linkTemplates`:

    a. Add a `variables` key following the OGC Records API whose values is a dictionary with entries like 

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

    b. Add a "uriTemplate" key that specifies how to contruct the resulting URL: i.e `http://hostname.tld/app/index.html?class=od&format={format}`

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
      "uriTemplate": "http://hostname.tld/app/index.html?class=od&expver={expver}",
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

| Field Name           | Type                      | Description                                                                                                           |
| -------------------- | ------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| uriTemplate          | URI Template              | Of the form "http://hostname.tld/app/index.html?class=od&expver={expver}", follows OGC Records Spec for uriTemplates  |
| variables            |                           |                                      |




### Additional Field Information

#### uriTemplate
Todo


#### variables
Todo
