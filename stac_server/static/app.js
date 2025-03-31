// app.js

// Take the query string and stick it on the API URL
function getSTACUrlFromQuery() {
  const params = new URLSearchParams(window.location.search);

  // get current window url and remove path part
  if (window.API_URL.startsWith("http")) {
    // Absolute URL: Use it directly
    api_url = new URL(window.API_URL);
  } else {
    // Relative URL: Combine with the current window's location
    api_url = new URL(window.location.href);
    api_url.pathname = window.API_URL;
  }

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

    const datePicker = item.querySelector("input[type='date']");
    if (datePicker) {
      values.push(datePicker.value.replace(/-/g, ""));
    }

    const timePicker = item.querySelector("input[type='time']");
    if (timePicker) {
      values.push(timePicker.value.replace(":", ""));
    }

    const enum_checkboxes = item.querySelectorAll(
      "input[type='checkbox']:checked"
    );
    if (enum_checkboxes.length > 0) {
      values.push(
        ...Array.from(enum_checkboxes).map((checkbox) => checkbox.value)
      );
    }

    const any = item.querySelector("input[type='text']");
    if (any && any.value !== "") {
      values.push(any.value);
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
      // If the key already exists,
      // and the values aren't already in there,
      // append the values
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
    // Update the item div with real content
    itemDiv.classList.remove("loading");

    const variables = link["variables"];
    const key = Object.keys(variables)[0];
    const variable = variables[key];

    // add data-key attribute to the itemDiv
    itemDiv.dataset.key = link.title;
    itemDiv.dataset.keyType = variable.type;

    itemDiv.innerHTML = `
      <h3 class="item-title">${link.title || "No title available"}</h3>
      <p class="item-type">Key Type: ${itemDiv.dataset.keyType || "Unknown"}</p>
      <p class="item-description">${
        variable.description ? variable.description.slice(0, 100) : ""
      }</p>
    `;

    if (variable.enum && variable.enum.length > 0) {
      const listContainer = renderCheckboxList(link);
      itemDiv.appendChild(listContainer);
    } else {
      const any = `<input type="text" name="${link.title}">`;
      const anyNode = document.createRange().createContextualFragment(any);
      itemDiv.appendChild(anyNode);
    }
  } catch (error) {
    console.error("Error loading item data:", error);
    itemDiv.innerHTML = `<p>Error loading item details: ${error}</p>`;
  }
}

function renderCheckboxList(link) {
  const variables = link["variables"];
  const key = Object.keys(variables)[0];
  const variable = variables[key];
  const value_descriptions = variable.value_descriptions || [];

  const listContainerHTML = `
      <div class="item-list-container">
        <div class="scrollable-list">
          ${variable.enum
            .map((value, index) => {
              const labelText = value_descriptions[index]
                ? `${value} - ${value_descriptions[index]}`
                : value;
              return `
                <div class="checkbox-container">
                  <label class="checkbox-label">
                  <input type="checkbox" class="item-checkbox" value="${value}" ${
                variable.enum.length === 1 ? "checked" : ""
              }>
                  ${labelText}
                  </label>
                </div>
              `;
            })
            .join("")}
        </div>
      </div>
    `;

  return document.createRange().createContextualFragment(listContainerHTML)
    .firstElementChild;
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

function renderRequestBreakdown(request, descriptions) {
  const container = document.getElementById("request-breakdown");
  const format_value = (key, value) => {
    return `<span class="value" title="${descriptions[key]["value_descriptions"][value]}">"${value}"</span>`;
  };

  const format_values = (key, values) => {
    if (values.length === 1) {
      return format_value(key, values[0]);
    }
    return `[${values.map((v) => format_value(key, v)).join(", ")}]`;
  };

  let html =
    `{\n` +
    request
      .map(
        ([key, values]) =>
          `    <span class="key" title="${
            descriptions[key]["description"]
          }">"${key}"</span>: ${format_values(key, values)},`
      )
      .join("\n") +
    `\n}`;
  container.innerHTML = html;
}

function renderRawSTACResponse(catalog) {
  const itemDetails = document.getElementById("raw-stac");
  // create new object without debug key
  let just_stac = Object.assign({}, catalog);
  delete just_stac.debug;
  itemDetails.textContent = JSON.stringify(just_stac, null, 2);

  const debug_container = document.getElementById("debug");
  debug_container.textContent = JSON.stringify(catalog.debug, null, 2);

  const qube_container = document.getElementById("qube");
  qube_container.innerHTML = catalog.debug.qube;
}

// Fetch STAC catalog and display items
async function fetchCatalog(request, stacUrl) {
  try {
    const response = await fetch(stacUrl);
    const catalog = await response.json();

    // Render the request breakdown in the sidebar
    renderRequestBreakdown(request, catalog.debug.descriptions);

    // Show the raw STAC in the sidebar
    renderRawSTACResponse(catalog);

    // Render the items from the catalog
    if (catalog.links) {
      console.log("Fetched STAC catalog:", stacUrl, catalog.links);
      renderCatalogItems(catalog.links);
    }

    // Highlight the request and raw STAC
    hljs.highlightElement(document.getElementById("raw-stac"));
    hljs.highlightElement(document.getElementById("debug"));
    hljs.highlightElement(document.getElementById("example-python"));
  } catch (error) {
    console.error("Error fetching STAC catalog:", error);
  }
}

// Initialize the viewer by fetching the STAC catalog
function initializeViewer() {
  const stacUrl = getSTACUrlFromQuery();
  const request = get_request_from_url();

  if (stacUrl) {
    console.log("Fetching STAC catalog from query string URL:", stacUrl);
    fetchCatalog(request, stacUrl);
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
