// app.js

// const API_BASE_URL = "http://127.0.0.1:8000/tree";

// Take the query string and stick it on the API URL
function getSTACUrlFromQuery() {
  const params = new URLSearchParams(window.location.search);

  // get current window url and remove path part
  let api_url = new URL(window.location.href);
  api_url.pathname = "/tree";

  for (const [key, value] of params.entries()) {
    api_url.searchParams.set(key, value);
  }

  console.log(api_url.toString());
  return api_url.toString();
}

function get_request_from_url() {
  // Extract the query params in order and split any with a , delimiter
  // request is an ordered array of [key, [value1, value2, value3, ...]]
  const url = new URL(window.location.href);
  const params = new URLSearchParams(url.search);
  const request = [];
  for (const [key, value] of params.entries()) {
    request.push([key, value.split(",")]);
  }
  return request;
}

function make_url_from_request(request) {
  const url = new URL(window.location.href);
  url.search = ""; // Clear existing params
  const params = new URLSearchParams();

  for (const [key, values] of request) {
    params.set(key, values.join(","));
  }
  url.search = params.toString();

  return url.toString().replace(/%2C/g, ",");
}

function goToPreviousUrl() {
  let request = get_request_from_url();
  request.pop();
  console.log("Request:", request);
  const url = make_url_from_request(request);
  console.log("URL:", url);
  window.location.href = make_url_from_request(request);
}

// Function to generate a new STAC URL based on current selection
function goToNextUrl() {
  const request = get_request_from_url();

  // Get the currently selected key = value,value2,value3 pairs
  const items = Array.from(document.querySelectorAll("div#items > div"));

  let any_new_keys = false;
  const new_keys = items.map((item) => {
    const key = item.dataset.key;
    const key_type = item.dataset.keyType;
    let values = [];

    if (key === "date") {
      const datePicker = item.querySelector("input[type='date']");
      //format date as YYYYMMDD
      values.push(datePicker.value.replace(/-/g, ""));
    } else if (key === "time") {
      const timePicker = item.querySelector("input[type='time']");
      //format time as HHMM
      console.log("replace", timePicker.value.replace(":", ""));
      values.push(timePicker.value.replace(":", ""));
    } else if (key_type === "enum") {
      values.push(
        ...Array.from(
          item.querySelectorAll("input[type='checkbox']:checked")
        ).map((checkbox) => checkbox.value)
      );
    } else {
      const any = item.querySelector("input[type='text']");
      if (any.value !== "") {
        values.push(any.value);
      }
    }

    // Keep track of whether any new keys are selected
    if (values.length > 0) {
      any_new_keys = true;
    }

    return { key, values };
  });

  // if not new keys are selected, do nothing
  if (!any_new_keys) {
    return;
  }

  // Update the request with the new keys
  for (const { key, values } of new_keys) {
    // Find the index of the existing key in the request array
    const existingIndex = request.findIndex(
      ([existingKey, existingValues]) => existingKey === key
    );

    if (existingIndex !== -1) {
      // If the key already exists, append the values
      request[existingIndex][1] = [...request[existingIndex][1], ...values];
    } else {
      // If the key doesn't exist, add a new entry
      request.push([key, values]);
    }
  }

  const url = make_url_from_request(request);
  window.location.href = url;
}

async function createCatalogItem(link, itemsContainer) {
  const itemDiv = document.createElement("div");
  itemDiv.className = "item loading";
  itemDiv.textContent = "Loading...";
  itemsContainer.appendChild(itemDiv);

  try {
    // Fetch details for each item/collection asynchronously
    let base_url = new URL(window.location.href);
    base_url.pathname = "/tree";
    let url = new URL(link.href, base_url);
    console.log("Fetching item details:", url);
    const response = await fetch(url);
    const itemData = await response.json();

    // Update the item div with real content
    itemDiv.classList.remove("loading");
    itemDiv.innerHTML = ""; // Clear "Loading..." text

    // add data-key attribute to the itemDiv
    itemDiv.dataset.key = itemData.id;
    itemDiv.dataset.keyType = itemData.key_type;

    const title = document.createElement("h3");
    title.className = "item-title";
    title.textContent = itemData.title || "No title available";
    itemDiv.appendChild(title);

    const key_type = document.createElement("p");
    key_type.className = "item-type";
    key_type.textContent = `Key Type: ${itemData.key_type || "Unknown"}`;
    itemDiv.appendChild(key_type);

    const optional = document.createElement("p");
    optional.className = "item-type";
    optional.textContent = `Optional: ${link.optional || "Unknown"}`;
    itemDiv.appendChild(optional);

    // const id = document.createElement("p");
    // id.className = "item-id";
    // id.textContent = `ID: ${itemData.id || link.href.split("/").pop()}`;
    // itemDiv.appendChild(id);

    const description = document.createElement("p");
    description.className = "item-description";
    const descText = itemData.description
      ? itemData.description.slice(0, 100)
      : "No description available";
    description.textContent = `${descText}...`;
    itemDiv.appendChild(description);

    if (itemData.key_type === "date" || itemData.key_type === "time") {
      // Render a date picker for the "date" key
      const picker = `<input type="${itemData.id}" name="${itemData.id}">`;
      //convert picker to HTML node
      const pickerNode = document
        .createRange()
        .createContextualFragment(picker);
      itemDiv.appendChild(pickerNode);
    }
    // Otherwise create a scrollable list with checkboxes for values if available
    else if (
      itemData.key_type === "enum" &&
      itemData.values &&
      itemData.values.length > 0
    ) {
      const listContainer = renderCheckboxList(itemData);
      itemDiv.appendChild(listContainer);
    } else {
      const any = `<input type="text" name="${itemData.id}">`;
      const anyNode = document.createRange().createContextualFragment(any);
      itemDiv.appendChild(anyNode);
    }
  } catch (error) {
    console.error("Error loading item data:", error);

    // In case of an error, display an error message
    itemDiv.innerHTML = "<p>Error loading item details</p>";
  }
}

// Render catalog items in the sidebar
function renderCatalogItems(links) {
  const itemsContainer = document.getElementById("items");
  itemsContainer.innerHTML = ""; // Clear previous items

  console.log("Number of Links:", links);
  const children = links.filter(
    (link) => link.rel === "child" || link.rel === "items"
  );
  console.log("Number of Children:", children.length);

  children.forEach((link) => {
    createCatalogItem(link, itemsContainer);
  });
}

// Fetch and display item details
async function loadItemDetails(url) {
  try {
    const resolved_url = new URL(url, API_BASE_URL);
    const response = await fetch(resolved_url);
    const item = await response.json();

    // Show details in the 'details' panel
    const itemDetails = document.getElementById("item-details");
    itemDetails.textContent = JSON.stringify(item, null, 2);
  } catch (error) {
    console.error("Error loading item details:", error);
  }
}

function show_resp_in_sidebar(catalog) {
  const itemDetails = document.getElementById("item-details");
  itemDetails.textContent = JSON.stringify(catalog, null, 2);
}

// Fetch STAC catalog and display items
async function fetchCatalog(stacUrl) {
  try {
    const response = await fetch(stacUrl);
    const catalog = await response.json();
    // Always load the most recently clicked item on the right-hand side
    show_resp_in_sidebar(catalog);

    // Render the items from the catalog
    if (catalog.links) {
      console.log("Fetched STAC catalog:", stacUrl, catalog.links);
      renderCatalogItems(catalog.links);
    }
  } catch (error) {
    console.error("Error fetching STAC catalog:", error);
  }
}

// Initialize the viewer by fetching the STAC catalog
function initializeViewer() {
  const stacUrl = getSTACUrlFromQuery();

  if (stacUrl) {
    console.log("Fetching STAC catalog from query string URL:", stacUrl);
    fetchCatalog(stacUrl);
  } else {
    console.error("No STAC URL provided in the query string.");
  }

  // Add event listener for the "Generate STAC URL" button
  const generateUrlBtn = document.getElementById("next-btn");
  generateUrlBtn.addEventListener("click", goToNextUrl);

  const previousUrlBtn = document.getElementById("previous-btn");
  previousUrlBtn.addEventListener("click", goToPreviousUrl);

  // Add event listener for the "Raw STAC" button
  const stacAnchor = document.getElementById("stac-anchor");
  stacAnchor.href = getSTACUrlFromQuery();
}

// Call initializeViewer on page load
initializeViewer();

function renderCheckboxList(itemData) {
  const listContainer = document.createElement("div");
  listContainer.className = "item-list-container";

  const listLabel = document.createElement("label");
  listLabel.textContent = "Select values:";
  listLabel.className = "list-label";

  const scrollableList = document.createElement("div");
  scrollableList.className = "scrollable-list";

  const checkboxesHtml = itemData.values
    .map((valueArray) => {
      const value = Array.isArray(valueArray) ? valueArray[0] : valueArray;
      const labelText = Array.isArray(valueArray)
        ? valueArray.join(" - ")
        : valueArray;
      return `
        <div class="checkbox-container">
          <input type="checkbox" class="item-checkbox" value="${value}">
          <label class="checkbox-label">${labelText}</label>
        </div>
      `;
    })
    .join("");

  scrollableList.innerHTML = checkboxesHtml;

  listContainer.appendChild(listLabel);
  listContainer.appendChild(scrollableList);
  return listContainer;
}
