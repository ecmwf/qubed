function toHTML(string) {
  return document.createRange().createContextualFragment(string)
    .firstElementChild;
}

// Take the query string and stick it on the API URL
function getSTACUrlFromQuery() {
  const params = new URLSearchParams(window.location.search);

  let api_url;
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

    console.log(`Checking ${key} ${key_type} and found ${values}`);
    return { key, values };
  });

  // if not new keys are selected, do nothing
  if (!any_new_keys) {
    return;
  }

  // Update the request with the new keys
  for (const { key, values } of new_keys) {
    if (values.length == 0) continue;

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
  if (Object.entries(link.variables)[0][1].on_frontier === false) {
    return;
  }

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

    function capitalize(val) {
      return String(val).charAt(0).toUpperCase() + String(val).slice(1);
    }

    itemDiv.innerHTML = `
      <h3 class="item-title">${
        capitalize(link.title) || "No title available"
      }</h3>
      <button class="all">*</button>
      <p class="item-type">Key Type: ${itemDiv.dataset.keyType || "Unknown"}</p>
      <p class="item-description">${
        variable.description ? variable.description.slice(0, 100) : ""
      }</p>
    `;

    if (false && key === "date") {
      console.log("Date", variable, exports);

      itemDiv.appendChild(toHTML("<input id='date-picker'></input>"));
      let dates = variable.enum;
      itemDiv.querySelector("button.all").style.display = "none";

      let picker = new AirDatepicker("#date-picker", {
        position: "bottom center",
        inline: true,
        locale: exports.default,
        range: true,
        multipleDatesSeparator: " - ",
        onRenderCell({ date, cellType }) {
          let isDay = cellType === "day",
            _date =
              String(date.getFullYear()).padStart(4, "0") +
              String(date.getMonth()).padStart(2, "0") +
              String(date.getDate()).padStart(2, "0"),
            shouldChangeContent = isDay && dates.includes(_date);

          return {
            classes: shouldChangeContent ? "has-data" : undefined,
          };
        },
      });
    } else if (variable.enum && variable.enum.length > 0) {
      const checkbox_list = renderCheckboxList(link);
      itemDiv.appendChild(checkbox_list);

      itemDiv.querySelector("button.all").addEventListener("click", () => {
        let new_state;
        if (checkbox_list.hasAttribute("disabled")) {
          checkbox_list.removeAttribute("disabled");
          itemDiv.querySelectorAll("input").forEach((c) => {
            c.removeAttribute("checked");
            c.removeAttribute("disabled");
          });
        } else {
          checkbox_list.setAttribute("disabled", "");
          itemDiv.querySelectorAll("input").forEach((c) => {
            c.setAttribute("checked", "true");
            c.setAttribute("disabled", "");
          });
        }
      });
    } else {
      const any = toHTML(`<input type="text" name="${link.title}">`);
      itemDiv.appendChild(any);
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
  const value_descriptions = variable.value_descriptions || {};

  function renderCheckbox(key, value, desc) {
    const id = `${key}=${value}`;
    let more_info = desc.url
      ? ` <a target=”_blank” class="more-info" href=${desc.url}>?<a>`
      : "";

    let human_label, code_label;
    if (desc.name) {
      human_label = `<label for="${id}">${desc.name}${more_info}</label>`;
      code_label = `<label class="code" for="${id}"><code>${value}</code></label>`;
    } else {
      human_label = `<label for="${id}">${value}${more_info}</label>`;
      code_label = `<label class="code"><code></code></label>`;
    }

    // Pre-check the box if there's only one option
    const checked = variable.enum.length === 1 ? "checked" : "";

    const checkbox = `<input type="checkbox" class="item-checkbox" value="${value}" id="${key}=${value}" ${checked}>`;

    return `
        <div class="checkbox-row">
        ${checkbox}
        ${human_label}
        ${code_label}
        </div>
    `;
  }

  const checkboxes = variable.enum
    .map((value) => renderCheckbox(key, value, value_descriptions[value] || {}))
    .join("");

  return toHTML(`<div class="checkbox-container">${checkboxes}</div>`);
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
