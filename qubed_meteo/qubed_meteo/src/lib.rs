pub mod adapters;

// Crate-root re-exports matching the flat API Omnicat's `catalogue_core`
// used to expose, so code that depended on it (now `catalogue_native` /
// `qubed_wasm`) only needs to repoint its `use` statements at this crate.
pub use adapters::catalogue::{
    Catalogue, KeyOrders, LocationRequests, MarsRequest, Selection as Request,
    coords_to_sorted_values, default_key_orders as dataset_key_orders, parse_request,
    request_to_selection,
};

/// [`adapters::catalogue::mars_requests`], with the dataset-ordering hint
/// always `"default"` (i.e. taken purely from `request`'s own `dataset` key,
/// if any) — the 3-argument shape `catalogue_core::mars_requests` used.
pub fn mars_requests(
    qube: &qubed::Qube,
    request: &Request,
    key_orders: &KeyOrders,
) -> Result<Vec<MarsRequest>, String> {
    adapters::catalogue::mars_requests(qube, request, key_orders, "default")
}

/// [`adapters::catalogue::mars_requests_by_location`], 3-argument shape —
/// see [`mars_requests`].
pub fn mars_requests_by_location(
    qube: &qubed::Qube,
    request: &Request,
    key_orders: &KeyOrders,
) -> Result<Vec<(String, Vec<MarsRequest>)>, String> {
    adapters::catalogue::mars_requests_by_location(qube, request, key_orders, "default")
}
