export function renderMapAndPickPolygon(containerId = "map") {
  const mapContainer = document.getElementById(containerId);
  if (!mapContainer) {
    console.error(`Container with id "${containerId}" not found.`);
    return;
  }
  mapContainer.innerHTML = "";
  const map = L.map(containerId).setView([37.7749, -122.4194], 13);
  L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
    attribution: '&copy; OpenStreetMap contributors'
  }).addTo(map);

  const drawnItems = new L.FeatureGroup();
  map.addLayer(drawnItems);

  const drawControl = new L.Control.Draw({
    draw: {
      polygon: true,
      marker: false,
      polyline: false,
      rectangle: false,
      circle: false,
      circlemarker: false
    },
    edit: { featureGroup: drawnItems }
  });
  map.addControl(drawControl);

  map.on(L.Draw.Event.CREATED, (event) => {
    const layer = event.layer;
    drawnItems.addLayer(layer);
    const coords = layer.getLatLngs()[0].map((latlng) => [latlng.lat, latlng.lng]);
    console.log("Polygon Coordinates:", coords);
    onPolygonPicked(coords);
  });
}

// can return and print chosen points
function onPolygonPicked(coords) {
  const output = document.getElementById("map_points");
  if (output) {
    output.textContent = JSON.stringify(coords, null, 2);
  } else {
    console.log("Polygon picked:", coords);
  }
}
