//! Adapter that builds a [`Qube`] from a JSON STAC (SpatioTemporal Asset Catalog) catalogue.
//!
//! Accepted input shapes:
//! - A STAC ItemCollection / FeatureCollection: `{"type": "FeatureCollection", "features": [...]}`
//! - A bare JSON array of STAC Items: `[{...}, {...}, ...]`
//! - A single STAC Item object: `{"type": "Feature", "id": ..., "assets": {...}, ...}`
//!
//! Every STAC **Asset** becomes a Qube leaf. A leaf's ancestry encodes the dimensions
//! that describe the STAC Item and its organizational context:
//!
//! ```text
//! collection -> platform -> datetime -> item -> asset
//! ```
//!
//! - `collection`: the Item's top-level `collection` field.
//! - `platform`: `properties.platform`.
//! - `datetime`: `properties.datetime` (falling back to `properties.start_datetime`).
//! - `item`: the Item's `id`.
//! - `asset`: the Asset's key within the Item's `assets` object.
//!
//! Any field that cannot be determined for a given item falls back to the literal
//! string `"unknown"` so the item/asset is still represented in the tree rather than
//! silently dropped.
//!
//! The leaf (asset) node's metadata, under the `"asset"` key, contains the
//! JSON-serialized original STAC Asset object verbatim, so no information from the
//! source catalogue is lost.

use qubed::{Coordinates, MetadataValues, NodeIdx, Qube};
use serde_json::Value;

/// Fallback value used whenever a dimension cannot be determined for an item.
const UNKNOWN: &str = "unknown";

/// Metadata key under which the original STAC Asset object (as a JSON string) is stored.
const ASSET_METADATA_KEY: &str = "asset";

pub trait FromStacCatalog {
    /// Build a [`Qube`] from a parsed STAC JSON document.
    ///
    /// See the [module-level docs](self) for the accepted input shapes and the
    /// resulting tree structure.
    fn from_stac_catalog(stac: &Value) -> Result<Qube, String>;
}

impl FromStacCatalog for Qube {
    fn from_stac_catalog(stac: &Value) -> Result<Qube, String> {
        let items = extract_items(stac)?;

        let mut qube = Qube::new();
        let root = qube.root();

        for item in items {
            add_item(&mut qube, root, item)?;
        }

        qube.compress();
        Ok(qube)
    }
}

/// Normalise the input into a list of STAC Item objects, regardless of whether the
/// top-level document is a FeatureCollection, a bare array of Items, or a single Item.
fn extract_items(stac: &Value) -> Result<Vec<&Value>, String> {
    match stac {
        Value::Array(items) => Ok(items.iter().collect()),
        Value::Object(map) => {
            if let Some(Value::Array(features)) = map.get("features") {
                // STAC ItemCollection / FeatureCollection.
                Ok(features.iter().collect())
            } else if map.get("assets").is_some() {
                // A single STAC Item.
                Ok(vec![stac])
            } else {
                Err(
                    "Unrecognized STAC JSON document: expected a 'features' array \
                     (ItemCollection), an 'assets' object (Item), or a bare JSON array of Items"
                        .to_string(),
                )
            }
        }
        _ => Err("STAC document must be a JSON object or array".to_string()),
    }
}

fn string_field<'a>(obj: &'a Value, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(|v| v.as_str())
}

fn property<'a>(item: &'a Value, key: &str) -> Option<&'a str> {
    item.get("properties").and_then(|p| p.get(key)).and_then(|v| v.as_str())
}

/// Create (or reuse) a single-valued string child node under `parent`.
fn child_with_string(
    qube: &mut Qube,
    key: &str,
    parent: NodeIdx,
    value: &str,
) -> Result<NodeIdx, String> {
    let mut coords = Coordinates::new();
    coords.append(value.to_string());
    qube.get_or_create_child(key, parent, Some(coords))
}

/// Add a single STAC Item (and all of its Assets) to the Qube under `root`.
fn add_item(qube: &mut Qube, root: NodeIdx, item: &Value) -> Result<(), String> {
    let collection = string_field(item, "collection").unwrap_or(UNKNOWN);
    let platform = property(item, "platform").unwrap_or(UNKNOWN);
    let datetime = property(item, "datetime")
        .or_else(|| property(item, "start_datetime"))
        .unwrap_or(UNKNOWN);
    let item_id = string_field(item, "id").unwrap_or(UNKNOWN);

    let assets = item
        .get("assets")
        .and_then(|v| v.as_object())
        .ok_or_else(|| format!("STAC item '{}' has no 'assets' object", item_id))?;

    let collection_node = child_with_string(qube, "collection", root, collection)?;
    let platform_node = child_with_string(qube, "platform", collection_node, platform)?;
    let datetime_node = child_with_string(qube, "datetime", platform_node, datetime)?;
    let item_node = child_with_string(qube, "item", datetime_node, item_id)?;

    for (asset_key, asset_value) in assets {
        let asset_node = child_with_string(qube, "asset", item_node, asset_key)?;

        let asset_json = serde_json::to_string(asset_value).map_err(|e| {
            format!("Failed to serialize asset '{}' of item '{}': {}", asset_key, item_id, e)
        })?;

        qube.set_metadata(
            asset_node,
            ASSET_METADATA_KEY,
            MetadataValues::from_strings(&[&asset_json]),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_item_collection() -> Value {
        json!({
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "id": "item-1",
                    "collection": "sentinel-2-l2a",
                    "properties": {
                        "platform": "sentinel-2a",
                        "datetime": "2024-01-01T00:00:00Z"
                    },
                    "assets": {
                        "B01": {"href": "https://example.com/item-1/B01.tif", "type": "image/tiff"},
                        "B02": {"href": "https://example.com/item-1/B02.tif", "type": "image/tiff"}
                    }
                },
                {
                    "type": "Feature",
                    "id": "item-2",
                    "collection": "sentinel-2-l2a",
                    "properties": {
                        "platform": "sentinel-2b",
                        "datetime": "2024-01-02T00:00:00Z"
                    },
                    "assets": {
                        "B01": {"href": "https://example.com/item-2/B01.tif", "type": "image/tiff"}
                    }
                }
            ]
        })
    }

    #[test]
    fn builds_qube_from_feature_collection() {
        let stac = sample_item_collection();
        let qube = Qube::from_stac_catalog(&stac).expect("should build qube");

        // 2 items, 3 assets total -> 3 leaf datacube paths.
        assert_eq!(qube.leaf_node_ids_paths().len(), 3);
    }

    #[test]
    fn builds_qube_from_single_item() {
        let stac = sample_item_collection();
        let item = stac["features"][0].clone();
        let qube = Qube::from_stac_catalog(&item).expect("should build qube");
        assert_eq!(qube.leaf_node_ids_paths().len(), 2);
    }

    #[test]
    fn builds_qube_from_bare_array() {
        let stac = sample_item_collection();
        let items = stac["features"].clone();
        let qube = Qube::from_stac_catalog(&items).expect("should build qube");
        assert_eq!(qube.leaf_node_ids_paths().len(), 3);
    }

    #[test]
    fn missing_fields_fall_back_to_unknown() {
        let stac = json!({
            "type": "Feature",
            "id": "lonely-item",
            "assets": {
                "data": {"href": "https://example.com/data.tif"}
            }
        });
        let qube = Qube::from_stac_catalog(&stac).expect("should build qube");
        assert_eq!(qube.leaf_node_ids_paths().len(), 1);
    }

    #[test]
    fn rejects_unrecognized_document() {
        let stac = json!({"type": "Catalog", "links": []});
        let result = Qube::from_stac_catalog(&stac);
        assert!(result.is_err());
    }
}
