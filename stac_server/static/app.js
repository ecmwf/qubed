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

    // Get text inputs but exclude the filter input
    const any = item.querySelector("input[type='text']:not(.filter-input)");
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
      <h3 class="item-title">${capitalize(link.title) || "No title available"
      }</h3>
      <button class="all">*</button>
      <p class="item-type">Key Type: ${itemDiv.dataset.keyType || "Unknown"}</p>
      <p class="item-description">${variable.description ? variable.description.slice(0, 100) : ""
      }</p>
    `;

    if (key === "date" && variable.enum && variable.enum.length > 30) {
      console.log("Date picker enabled");
      console.log("First few dates:", variable.enum.slice(0, 10));

      // Create a unique ID for this date picker
      const pickerId = `date-picker-${link.title}`;
      const hiddenInputId = `date-input-${link.title}`;

      itemDiv.appendChild(toHTML(`<input id='${pickerId}' class='date-picker-input'></input>`));
      itemDiv.appendChild(toHTML(`<input type='text' id='${hiddenInputId}' style='display:none;' name='${link.title}'></input>`));
      itemDiv.appendChild(toHTML(`<div class='date-picker-hint' id='${pickerId}-hint'>💡 Click a date twice to select it individually, or click two different dates to select a range.</div>`));

      let dates = variable.enum.map(d => String(d));
      itemDiv.querySelector("button.all").style.display = "none";

      // Create a set for fast lookup (normalize to YYYY-MM-DD format)
      const availableDatesSet = new Set(dates);
      console.log("Available dates set size:", availableDatesSet.size);

      // Parse dates from enum to get min and max dates
      let parsedDates = dates.map(d => {
        // Handle both formats: "YYYY-MM-DD" or "YYYYMMDD"
        const dateStr = String(d);
        if (dateStr.includes('-')) {
          return new Date(dateStr);
        } else {
          const year = parseInt(dateStr.substring(0, 4));
          const month = parseInt(dateStr.substring(4, 6)) - 1;
          const day = parseInt(dateStr.substring(6, 8));
          return new Date(year, month, day);
        }
      });

      let minDate = new Date(Math.min(...parsedDates));
      let maxDate = new Date(Math.max(...parsedDates));

      console.log("Date range:", minDate.toISOString(), "to", maxDate.toISOString());

      // Track selected dates manually for better control
      let manuallySelectedDates = new Set();
      let lastClickedDate = null;

      let picker = new AirDatepicker(`#${pickerId}`, {
        position: "bottom center",
        inline: true,
        locale: exports.default,
        multipleDates: true,
        multipleDatesSeparator: ",",
        minDate: minDate,
        maxDate: maxDate,
        onSelect({ date, formattedDate, datepicker }) {
          // Prevent default behavior - we'll handle selection manually
        },
        onRenderCell({ date, cellType }) {
          if (cellType === "day") {
            const year = date.getFullYear();
            const month = String(date.getMonth() + 1).padStart(2, '0');
            const day = String(date.getDate()).padStart(2, '0');
            // Check in the format that matches the input
            let dateStr;
            if (dates[0].includes('-')) {
              dateStr = `${year}-${month}-${day}`;
            } else {
              dateStr = `${year}${month}${day}`;
            }
            const hasData = availableDatesSet.has(dateStr);

            return {
              classes: hasData ? "has-data" : "",
              disabled: !hasData,
            };
          }
          return {};
        },
      });

      // Custom click handler for date cells
      const hintElement = document.getElementById(`${pickerId}-hint`);

      // Wait for datepicker to render, then attach event handler
      setTimeout(() => {
        const datepickerContainer = document.querySelector(`#${pickerId}`).parentElement.querySelector('.air-datepicker');

        if (datepickerContainer) {
          datepickerContainer.addEventListener('click', (e) => {
            const cell = e.target.closest('.air-datepicker-cell.-day-');
            if (!cell || cell.classList.contains('-disabled-')) return;

            // Get the date from the cell's data attributes
            const dayNumber = cell.getAttribute('data-date');
            const monthNumber = cell.getAttribute('data-month');
            const yearNumber = cell.getAttribute('data-year');

            if (!dayNumber || !monthNumber || !yearNumber) return;

            const cellDate = new Date(parseInt(yearNumber), parseInt(monthNumber), parseInt(dayNumber));

            const formatDate = (d) => {
              const year = d.getFullYear();
              const month = String(d.getMonth() + 1).padStart(2, '0');
              const day = String(d.getDate()).padStart(2, '0');
              if (dates[0].includes('-')) {
                return `${year}-${month}-${day}`;
              } else {
                return `${year}${month}${day}`;
              }
            };

            const clickedDateStr = formatDate(cellDate);

            // Check if this date has data
            if (!availableDatesSet.has(clickedDateStr)) return;

            const isSameAsPrevious = lastClickedDate &&
                                     cellDate.getTime() === lastClickedDate.getTime();

            if (isSameAsPrevious) {
              // Clicking same date twice - toggle individual date
              if (manuallySelectedDates.has(clickedDateStr)) {
                manuallySelectedDates.delete(clickedDateStr);
                console.log("Removed date:", clickedDateStr);
                if (hintElement) hintElement.textContent = `🗑️ Removed ${clickedDateStr}. Total: ${manuallySelectedDates.size} dates selected.`;
              } else {
                manuallySelectedDates.add(clickedDateStr);
                console.log("Added single date:", clickedDateStr);
                if (hintElement) hintElement.textContent = `✅ Added ${clickedDateStr}. Total: ${manuallySelectedDates.size} dates selected.`;
              }
              lastClickedDate = null; // Reset for next selection
            } else if (lastClickedDate) {
              // Two different dates clicked - create a range
              const [startDate, endDate] = [lastClickedDate, cellDate].sort((a, b) => a - b);

              console.log("Creating range from", formatDate(startDate), "to", formatDate(endDate));

              let currentDate = new Date(startDate);
              const rangeEnd = new Date(endDate);
              let rangeCount = 0;

              while (currentDate <= rangeEnd) {
                const dateStr = formatDate(currentDate);
                if (availableDatesSet.has(dateStr)) {
                  manuallySelectedDates.add(dateStr);
                  rangeCount++;
                }
                currentDate.setDate(currentDate.getDate() + 1);
              }

              console.log("Range added, total dates selected:", manuallySelectedDates.size);
              if (hintElement) hintElement.textContent = `📅 Added range: ${rangeCount} dates. Total: ${manuallySelectedDates.size} dates selected.`;
              lastClickedDate = null; // Reset for next selection
            } else {
              // First click of a potential range
              lastClickedDate = cellDate;
              console.log("First date clicked for range:", clickedDateStr);
              if (hintElement) hintElement.textContent = `🎯 First date selected: ${clickedDateStr}. Click another date to create a range, or click this date again to select it individually.`;
              return; // Don't update selection yet, wait for second click
            }

            // Update the visual selection in datepicker
            const selectedDateObjects = Array.from(manuallySelectedDates).map(dateStr => {
              if (dateStr.includes('-')) {
                return new Date(dateStr);
              } else {
                const year = parseInt(dateStr.substring(0, 4));
                const month = parseInt(dateStr.substring(4, 6)) - 1;
                const day = parseInt(dateStr.substring(6, 8));
                return new Date(year, month, day);
              }
            });

            picker.selectDate(selectedDateObjects);

            // Update hidden input
            const hiddenInput = document.getElementById(hiddenInputId);
            hiddenInput.value = Array.from(manuallySelectedDates).join(',');
            console.log("Total selected dates:", manuallySelectedDates.size);
            console.log("Selected dates:", hiddenInput.value.split(',').slice(0, 10).join(', '), '...');
          });
        }
      }, 100);

      console.log("Datepicker initialized");
    } else if (variable.enum && variable.enum.length > 0) {
      // Add filter input at the top if there are many options
      if (variable.enum.length > 5) {
        const filterWrapper = toHTML(`
          <div class="filter-wrapper">
            <input type="text" class="filter-input" id="filter-${link.title}" placeholder="🔍 Filter options...">
          </div>
        `);
        itemDiv.appendChild(filterWrapper);
      }

      // Add checkbox list
      const checkbox_list = renderCheckboxList(link);
      itemDiv.appendChild(checkbox_list);

      // Add filter functionality if filter exists
      if (variable.enum.length > 5) {
        const filterInput = itemDiv.querySelector(`#filter-${link.title}`);
        if (filterInput) {
          filterInput.addEventListener('input', (e) => {
            const filterText = e.target.value.toLowerCase();
            const checkboxRows = checkbox_list.querySelectorAll('.checkbox-row');

            checkboxRows.forEach(row => {
              const label = row.querySelector('label');
              const code = row.querySelector('label.code code');
              const labelText = label ? label.textContent.toLowerCase() : '';
              const codeText = code ? code.textContent.toLowerCase() : '';

              if (labelText.includes(filterText) || codeText.includes(filterText)) {
                row.style.display = '';
              } else {
                row.style.display = 'none';
              }
            });
          });
        }
      }

      itemDiv.querySelector("button.all").addEventListener("click", () => {
        let new_state;
        if (checkbox_list.hasAttribute("disabled")) {
          checkbox_list.removeAttribute("disabled");
          itemDiv.querySelectorAll("input[type='checkbox']").forEach((c) => {
            c.removeAttribute("checked");
            c.removeAttribute("disabled");
          });
        } else {
          checkbox_list.setAttribute("disabled", "");
          itemDiv.querySelectorAll("input[type='checkbox']").forEach((c) => {
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
      ? ` <a target="_blank" class="more-info" href="${desc.url}">?</a>`
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
    return `<span class="punct">"</span><span class="value" title="${descriptions[key]["value_descriptions"][value]}">${value}</span><span class="punct">"</span>`;
  };

  const format_values = (key, values) => {
    if (values.length === 1) {
      return format_value(key, values[0]);
    }
    return `<span class="punct">[</span>${values.map((v) => format_value(key, v)).join(`<span class="punct">,</span> `)}<span class="punct">]</span>`;
  };

  let html =
    `<span class="punct">{</span>\n` +
    request
      .map(
        ([key, values]) =>
          `    <span class="punct">"</span><span class="key" title="${descriptions[key]["description"]
          }">${key}</span><span class="punct">"</span><span class="punct">:</span> ${format_values(key, values)}<span class="punct">,</span>`
      )
      .join("\n") +
    `\n<span class="punct">}</span>`;
  container.innerHTML = html;
}

function renderMARSRequest(request, descriptions) {
  const container = document.getElementById("final_req");
  
  console.log("=== renderMARSRequest START ===");
  console.log("request:", request);
  console.log("request type:", typeof request);
  console.log("is array?", Array.isArray(request));
  console.log("descriptions:", descriptions);
  
  if (!Array.isArray(request)) {
    console.error("ERROR: request is not an array!", request);
    container.innerHTML = `<p style="color: red;">ERROR: request is not an array. Got: ${typeof request}</p><pre>${JSON.stringify(request, null, 2)}</pre>`;
    return;
  }
  
  if (request.length === 0) {
    console.warn("WARNING: request array is empty");
    container.innerHTML = `<p style="color: orange;">No MARS requests generated</p>`;
    return;
  }
  
  console.log("First request item:", request[0]);
  console.log("First item entries:", Object.entries(request[0]));
  
  const format_value = (key, value) => {
    // Convert value to string if it's not already
    const stringValue = String(value);
    const desc = descriptions?.[key]?.["value_descriptions"]?.[stringValue];
    return `<span class="punct">"</span><span class="value" title="${desc || ''}">${stringValue}</span><span class="punct">"</span>`;
  };

  const format_values = (key, values) => {
    console.log(`format_values called: key=${key}, values=${values}, type=${typeof values}`);
    
    // Handle different types of values
    if (values === null || values === undefined) {
      return `<span class="punct">null</span>`;
    }
    
    // Check if it's an array-like structure
    let valueArray;
    if (Array.isArray(values)) {
      console.log(`  -> is array, length ${values.length}`);
      valueArray = values;
    } else if (typeof values === 'object') {
      console.log(`  -> is object, stringify`);
      // If it's an object, just JSON stringify it
      return `<span class="value">${JSON.stringify(values)}</span>`;
    } else {
      console.log(`  -> is scalar (${typeof values}), wrapping in array`);
      // Scalar value - wrap in array
      valueArray = [values];
    }
    
    // If array is empty, return empty array repr
    if (valueArray.length === 0) {
      return `<span class="punct">[]</span>`;
    }
    
    // If array has single element, just return that
    if (valueArray.length === 1) {
      const result = format_value(key, valueArray[0]);
      console.log(`  -> returning single value: ${result}`);
      return result;
    }
    
    // Multiple values - return as array
    return `<span class="punct">[</span>${valueArray.map((v) => format_value(key, v)).join(`<span class="punct">,</span> `)}<span class="punct">]</span>`;
  };

  // Add feature object to each request if polygon is selected
  const requestsWithFeature = selectedPolygon ? request.map(obj => ({
    ...obj,
    feature: {
      type: "polygon",
      shape: selectedPolygon
    }
  })) : request;

  // Store for copying
  currentMARSRequests = requestsWithFeature;

  try {
    let html =
    `<span class="punct">[</span>\n` +
    requestsWithFeature
      .map((obj, objIdx) => {
        console.log(`Rendering object ${objIdx}:`, obj);
        const entries = Object.entries(obj);
        return `  <span class="punct">{</span>\n` +
        entries
          .map(
            ([key, values], idx) => {
              const isLast = idx === entries.length - 1;
              if (key === "feature" && values && typeof values === "object" && values.type === "polygon") {
                // Format the feature object specially
                const shapeStr = JSON.stringify(values.shape, null, 0);
                return `    <span class="punct">"</span><span class="key">feature</span><span class="punct">"</span><span class="punct">:</span> <span class="punct">{</span>\n` +
                       `      <span class="punct">"</span><span class="key">type</span><span class="punct">"</span><span class="punct">:</span> <span class="punct">"</span><span class="value">${values.type}</span><span class="punct">"</span><span class="punct">,</span>\n` +
                       `      <span class="punct">"</span><span class="key">shape</span><span class="punct">"</span><span class="punct">:</span> <span class="value">${shapeStr}</span>\n` +
                       `    <span class="punct">}</span>${isLast ? '' : '<span class="punct">,</span>'}`;
              }
              const formattedValue = format_values(key, values);
              return `    <span class="punct">"</span><span class="key" title="${descriptions?.[key]?.description || ""}">${key}</span><span class="punct">"</span><span class="punct">:</span> ${formattedValue}${isLast ? '' : '<span class="punct">,</span>'}`;
            }
          )
          .join("\n") +
        `\n  <span class="punct">}</span>`;
      })
      .join(`<span class="punct">,</span>\n`) +
    `\n<span class="punct">]</span>`;
    container.innerHTML = html;
    console.log("=== renderMARSRequest COMPLETED SUCCESSFULLY ===");
  } catch (error) {
    console.error("=== ERROR in renderMARSRequest ===", error);
    console.error("Stack trace:", error.stack);
    const container = document.getElementById("final_req");
    container.innerHTML = `<p style="color: red;">Error rendering MARS requests: ${error.message}</p><pre>${JSON.stringify(request, null, 2)}</pre><pre>${error.stack}</pre>`;
  }
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
    let catalog;
    if (window.__wasmCatalogue) {
      // Use the client-side Rust/WASM catalogue — no network round-trip needed.
      // `request` is an ordered array of [key, [value, ...]] pairs.
      const reqObj = Object.fromEntries(request);
      catalog = JSON.parse(window.__wasmCatalogue.stac(JSON.stringify(reqObj)));
      console.log("[wasm] WASM stac() returned catalog:", catalog);
    } else {
      const response = await fetch(stacUrl);
      catalog = await response.json();
    }

    console.log("Fetched catalog:", catalog);

    // Check if we've reached the end of the catalogue (final_object has data)
    const hasReachedEnd = catalog.final_object && catalog.final_object.length > 0;

    console.log("Has reached end:", hasReachedEnd, "final_object:", catalog.final_object);

    // Get section elements
    const currentSelectionSection = document.getElementById("current-selection-section");
    const marsRequestsSection = document.getElementById("mars-requests-section");
    const nextButton = document.getElementById("next-btn");

    if (hasReachedEnd) {
      // At the end: show MARS requests, hide current selection and next button
      console.log("At end of traversal, rendering MARS requests");
      currentSelectionSection.style.display = "none";
      marsRequestsSection.style.display = "block";
      nextButton.style.display = "none";
      catalogCache = catalog; // Store catalog for re-rendering with features
      console.log("Descriptions available:", catalog.debug.descriptions);
      renderMARSRequest(catalog.final_object, catalog.debug.descriptions);
    } else {
      // Not at the end: show current selection, hide MARS requests, show next button
      currentSelectionSection.style.display = "block";
      marsRequestsSection.style.display = "none";
      nextButton.style.display = "flex";
      renderRequestBreakdown(request, catalog.debug.descriptions);
    }

    // Show the raw STAC in the sidebar
    renderRawSTACResponse(catalog);

    // Render the items from the catalog
    if (catalog.links) {
      console.log("Fetched STAC catalog:", stacUrl, catalog.links);
      renderCatalogItems(catalog.links);
    }

    // Show region selection at the end of catalogue
    const regionSelection = document.getElementById("region-selection");
    const catalogList = document.getElementById("catalog-list");
    const polytopeSection = document.getElementById("polytope-section");
    if (hasReachedEnd) {
      regionSelection.style.display = "block";
      catalogList.classList.add("region-active");
      if (polytopeSection) polytopeSection.style.display = "block";
    } else {
      regionSelection.style.display = "none";
      catalogList.classList.remove("region-active");
      if (polytopeSection) polytopeSection.style.display = "none";
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
  window.__viewerStarted = true;
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

// Copy MARS requests to clipboard
function copyMARSRequests() {
  const copyBtn = document.getElementById("copy-mars-btn");
  const btnText = copyBtn.querySelector(".copy-btn-text");

  // Use the stored MARS requests with feature if available
  const jsonContent = JSON.stringify(currentMARSRequests, null, 2);

  navigator.clipboard.writeText(jsonContent).then(() => {
    // Change button text temporarily
    btnText.textContent = "Copied!";
    copyBtn.classList.add("copied");

    // Reset after 2 seconds
    setTimeout(() => {
      btnText.textContent = "Copy";
      copyBtn.classList.remove("copied");
    }, 2000);
  }).catch(err => {
    console.error("Failed to copy:", err);
    btnText.textContent = "Failed";
    setTimeout(() => {
      btnText.textContent = "Copy";
    }, 2000);
  });
}

// Download JSON data as a file
function downloadJSON(data, filename) {
  const jsonString = JSON.stringify(data, null, 2);
  const blob = new Blob([jsonString], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

// ============================================
// Geographic Region Selection with Map
// ============================================

let regionMap = null;
let drawnItems = null;
let selectedPolygon = null;
let currentMARSRequests = []; // Store current MARS requests for copying
let catalogCache = null; // Store catalog for re-rendering when polygon changes

function initializeRegionMap() {
  const mapElement = document.getElementById('map');
  if (!mapElement || regionMap) return;

  // Initialize map centered on the world
  regionMap = L.map('map').setView([20, 0], 2);

  // Add OpenStreetMap tile layer
  L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
    attribution: '© OpenStreetMap contributors',
    maxZoom: 18,
  }).addTo(regionMap);

  // Initialize the FeatureGroup to store editable layers
  drawnItems = new L.FeatureGroup();
  regionMap.addLayer(drawnItems);

  // Initialize the draw control
  const drawControl = new L.Control.Draw({
    position: 'topright',
    draw: {
      polyline: false,
      circle: false,
      circlemarker: false,
      marker: false,
      rectangle: true,
      polygon: {
        allowIntersection: false,
        showArea: true,
        shapeOptions: {
          color: '#0066cc',
          weight: 2,
          fillOpacity: 0.2
        }
      }
    },
    edit: {
      featureGroup: drawnItems,
      remove: true
    }
  });
  regionMap.addControl(drawControl);

  // Handle polygon creation
  regionMap.on('draw:created', function (e) {
    // Clear previous polygons
    drawnItems.clearLayers();

    const layer = e.layer;
    drawnItems.addLayer(layer);

    // Get the coordinates
    const coordinates = layer.getLatLngs()[0].map(latlng => [
      parseFloat(latlng.lat.toFixed(6)),
      parseFloat(latlng.lng.toFixed(6))
    ]);

    // Close the polygon by adding the first point at the end
    coordinates.push(coordinates[0]);

    selectedPolygon = coordinates;
    displaySelectedRegion(coordinates);
  });

  // Handle polygon edit
  regionMap.on('draw:edited', function (e) {
    const layers = e.layers;
    layers.eachLayer(function (layer) {
      const coordinates = layer.getLatLngs()[0].map(latlng => [
        parseFloat(latlng.lat.toFixed(6)),
        parseFloat(latlng.lng.toFixed(6))
      ]);
      coordinates.push(coordinates[0]);
      selectedPolygon = coordinates;
      displaySelectedRegion(coordinates);
    });
  });

  // Handle polygon deletion
  regionMap.on('draw:deleted', function (e) {
    selectedPolygon = null;
    document.getElementById('selected-region').style.display = 'none';
  });

  // Force map to resize properly
  setTimeout(() => {
    regionMap.invalidateSize();
  }, 100);
}

function displaySelectedRegion(coordinates) {
  const selectedRegionDiv = document.getElementById('selected-region');
  const coordinatesDisplay = document.getElementById('region-coordinates');

  const regionFeature = {
    type: "polygon",
    shape: coordinates
  };

  coordinatesDisplay.textContent = JSON.stringify({ feature: regionFeature }, null, 2);
  selectedRegionDiv.style.display = 'block';

  // Re-render MARS requests with the feature appended
  if (catalogCache && catalogCache.final_object) {
    renderMARSRequest(catalogCache.final_object, catalogCache.debug.descriptions);
  }
}

// Event listeners for region selection
document.addEventListener("DOMContentLoaded", () => {
  const enableRegionBtn = document.getElementById('enable-region-btn');
  const skipRegionBtn = document.getElementById('skip-region-btn');
  const clearRegionBtn = document.getElementById('clear-region-btn');
  const mapContainer = document.getElementById('map-container');

  if (enableRegionBtn) {
    enableRegionBtn.addEventListener('click', () => {
      mapContainer.style.display = 'block';
      enableRegionBtn.style.display = 'none';
      skipRegionBtn.textContent = 'Continue Without Region';
      initializeRegionMap();
    });
  }

  if (skipRegionBtn) {
    skipRegionBtn.addEventListener('click', () => {
      // User chose to skip region selection - could proceed to next step
      console.log('User skipped region selection');
      // Here you could trigger the next action or inform the user
    });
  }

  if (clearRegionBtn) {
    clearRegionBtn.addEventListener('click', () => {
      if (drawnItems) {
        drawnItems.clearLayers();
      }
      selectedPolygon = null;
      document.getElementById('selected-region').style.display = 'none';

      // Re-render MARS requests without the feature
      if (catalogCache && catalogCache.final_object) {
        renderMARSRequest(catalogCache.final_object, catalogCache.debug.descriptions);
      }
    });
  }
});

// ============================================
// Polytope Query Handler
// ============================================

async function queryPolytope() {
  const polytopeBtn = document.getElementById('polytope-btn');
  const polytopeBtnText = document.getElementById('polytope-btn-text');
  const polytopeStatus = document.getElementById('polytope-status');
  const polytopeResults = document.getElementById('polytope-results');
  const emailInput = document.getElementById('polytope-email');
  const keyInput = document.getElementById('polytope-key');

  if (!currentMARSRequests || currentMARSRequests.length === 0) {
    polytopeStatus.textContent = 'No MARS requests available to query.';
    polytopeStatus.className = 'polytope-status error';
    polytopeStatus.style.display = 'block';
    return;
  }

  // Validate credentials
  const email = emailInput.value.trim();
  const apiKey = keyInput.value.trim();

  if (!email || !apiKey) {
    polytopeStatus.textContent = 'Please provide both email and API key to query Polytope.';
    polytopeStatus.className = 'polytope-status error';
    polytopeStatus.style.display = 'block';
    return;
  }

  // Disable button and show loading state
  polytopeBtn.disabled = true;
  polytopeBtnText.textContent = 'Querying...';
  polytopeStatus.textContent = `Submitting ${currentMARSRequests.length} request(s) to Polytope service...`;
  polytopeStatus.className = 'polytope-status loading';
  polytopeStatus.style.display = 'block';
  polytopeResults.innerHTML = '';
  polytopeResults.style.display = 'none';

  try {
    const response = await fetch('/api/v2/polytope/query', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        requests: currentMARSRequests,
        credentials: {
          user_email: email,
          user_key: apiKey
        }
      }),
    });

    const result = await response.json();

    if (!response.ok) {
      throw new Error(result.detail || 'Failed to query Polytope service');
    }

    // Show success message
    polytopeStatus.textContent = `Successfully submitted ${result.total} request(s). ${result.successful} succeeded, ${result.failed} failed.`;
    polytopeStatus.className = 'polytope-status success';

    // Store results globally for notebook access
    window.polytopeResults = result.results;

    // Display detailed results
    if (result.results && result.results.length > 0) {
      polytopeResults.innerHTML = result.results.map((res, idx) => `
        <div class="polytope-result-item ${res.success ? 'success' : 'error'}">
          <div class="polytope-result-header">
            Request ${idx + 1}: ${res.success ? '✓ Success' : '✗ Failed'}
          </div>
          <div class="polytope-result-detail">
            ${res.success
              ? `Data retrieved successfully${res.data_size ? ` (${res.data_size})` : ''}`
              : `Error: ${res.error || 'Unknown error'}`
            }
          </div>
          ${res.message ? `<div class="polytope-result-detail">${res.message}</div>` : ''}
          ${res.success && res.json_data ? `
            <div style="display: flex; gap: 0.5rem; margin-top: 0.5rem; flex-wrap: wrap;">
              <button class="download-json-btn" data-request-idx="${idx}" style="padding: 0.4rem 0.8rem; background: var(--primary-color); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 0.9rem;">
                📥 Download JSON
              </button>
              <button class="open-notebook-btn" data-request-idx="${idx}" style="padding: 0.4rem 0.8rem; font-size: 0.9rem;">
                <svg width="16" height="16" viewBox="0 0 20 20" fill="currentColor">
                  <path d="M9 4.804A7.968 7.968 0 005.5 4c-1.255 0-2.443.29-3.5.804v10A7.969 7.969 0 015.5 14c1.669 0 3.218.51 4.5 1.385A7.962 7.962 0 0114.5 14c1.255 0 2.443.29 3.5.804v-10A7.968 7.968 0 0014.5 4c-1.255 0-2.443.29-3.5.804V12a1 1 0 11-2 0V4.804z"/>
                </svg>
                Open in Notebook
              </button>
            </div>
          ` : ''}
        </div>
      `).join('');
      polytopeResults.style.display = 'block';

      // Add event listeners to download buttons
      document.querySelectorAll('.download-json-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
          const idx = parseInt(e.target.getAttribute('data-request-idx'));
          const resultData = result.results[idx];
          if (resultData && resultData.json_data) {
            downloadJSON(resultData.json_data, `polytope_request_${idx + 1}.json`);
          }
        });
      });

      // Add event listeners to notebook buttons
      document.querySelectorAll('.open-notebook-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
          const idx = parseInt(e.target.closest('.open-notebook-btn').getAttribute('data-request-idx'));
          const resultData = result.results[idx];
          if (resultData && resultData.json_data) {
            openInNotebook(resultData.json_data, idx);
          }
        });
      });
    }

    polytopeBtnText.textContent = 'Query Complete';
  } catch (error) {
    console.error('Polytope query error:', error);
    polytopeStatus.textContent = `Error: ${error.message}`;
    polytopeStatus.className = 'polytope-status error';
  } finally {
    // Re-enable button after a delay
    setTimeout(() => {
      polytopeBtn.disabled = false;
      polytopeBtnText.textContent = 'Query Polytope Service';
    }, 2000);
  }
}

// ============================================
// JupyterLite Notebook Integration
// ============================================

let codeEditor = null;
let currentNotebookData = null;

// Server-side execution - no Pyodide initialization needed
// Python code runs on the server with full package support

function initCodeEditor() {
  if (codeEditor) {
    return codeEditor;
  }

  const editorElement = document.getElementById('code-editor');
  codeEditor = CodeMirror(editorElement, {
    value: getDefaultNotebookCode(),
    mode: 'python',
    theme: 'monokai',
    lineNumbers: true,
    indentUnit: 4,
    tabSize: 4,
    indentWithTabs: false,
    lineWrapping: true,
  });

  return codeEditor;
}

function getDefaultNotebookCode() {
  return `# Polytope Data Visualization - Request 1
# The data is available in the 'polytope_data' variable

import json
import numpy as np
import covjsonkit
import earthkit.plots

from covjsonkit.api import Covjsonkit

decoder = Covjsonkit().decode(polytope_data)

ds = decoder.to_xarray()

print(ds)

# Handle missing/masked values
if '2t' in ds:
    data = ds['2t']
    # Replace NaN with a fill value or drop them
    data_filled = data.where(~np.isnan(data), drop=True)

    chart = earthkit.plots.Map(domain="Germany")
    chart.point_cloud(
        data_filled,
        x="longitude",
        y="latitude",
        auto_style=True
    )

    chart.coastlines()
    chart.borders()
    chart.gridlines()

    chart.title("{variable_name} (number={number})")

    chart.legend()
else:
    print("Variable '2t' not found in dataset")
    print("Available variables:", list(ds.data_vars))

# chart.show()  # Not needed - figure is captured automatically
`;
}

async function openInNotebook(jsonData, requestIdx) {
  const notebookSection = document.getElementById('notebook-section');

  // Show notebook section
  notebookSection.style.display = 'block';
  notebookSection.scrollIntoView({ behavior: 'smooth', block: 'start' });

  // Store data globally
  currentNotebookData = jsonData;
  window.currentNotebookRequestIdx = requestIdx;

  // Initialize code editor if not already done
  if (!codeEditor) {
    initCodeEditor();
  }

  // Update the default code with the request index
  const defaultCode = `# Polytope Data Visualization - Request ${requestIdx + 1}
# The data is available in the 'polytope_data' variable

import json
import numpy as np
import covjsonkit
import earthkit.plots

from covjsonkit.api import Covjsonkit

decoder = Covjsonkit().decode(polytope_data)

ds = decoder.to_xarray()

print(ds)

# Handle missing/masked values
if '2t' in ds:
    data = ds['2t']
    # Replace NaN with a fill value or drop them
    data_filled = data.where(~np.isnan(data), drop=True)

    chart = earthkit.plots.Map(domain="Germany")
    chart.point_cloud(
        data_filled,
        x="longitude",
        y="latitude",
        auto_style=True
    )

    chart.coastlines()
    chart.borders()
    chart.gridlines()

    chart.title("{variable_name} (number={number})")

    chart.legend()
else:
    print("Variable '2t' not found in dataset")
    print("Available variables:", list(ds.data_vars))

# chart.show()  # Not needed - figure is captured automatically
`;

  codeEditor.setValue(defaultCode);
}

async function runPythonCode() {
  const runBtn = document.getElementById('run-code-btn');
  const runBtnText = document.getElementById('run-code-text');
  const outputDiv = document.getElementById('notebook-output');
  const outputContent = document.getElementById('output-content');
  const outputImages = document.getElementById('output-images');
  const loadingDiv = document.getElementById('notebook-loading-exec');

  if (!currentNotebookData) {
    outputContent.textContent = 'Error: No data available. Please query Polytope first.';
    outputDiv.style.display = 'block';
    return;
  }

  // Disable button and show loading
  runBtn.disabled = true;
  runBtnText.textContent = 'Executing...';
  loadingDiv.style.display = 'flex';
  outputDiv.style.display = 'none';

  try {
    // Get code from editor
    const code = codeEditor.getValue();

    // Send code to server for execution
    const response = await fetch('/api/v2/execute', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        code: code,
        data: currentNotebookData
      })
    });

    const result = await response.json();

    if (result.success) {
      // Display output from stdout and stderr
      let output = result.stdout || '';
      if (result.stderr) {
        output += '\n\nErrors/Warnings:\n' + result.stderr;
      }
      outputContent.textContent = output || '(No output)';

      // Display images if any
      outputImages.innerHTML = '';
      if (result.images && result.images.length > 0) {
        result.images.forEach((imgBase64, idx) => {
          const img = document.createElement('img');
          img.src = `data:image/png;base64,${imgBase64}`;
          img.alt = `Plot ${idx + 1}`;
          img.className = 'output-image';
          outputImages.appendChild(img);
        });
      }
    } else {
      // Display error
      outputContent.textContent = `Error (${result.error_type || 'Error'}): ${result.error}`;
      outputImages.innerHTML = '';
    }

    outputDiv.style.display = 'block';
    loadingDiv.style.display = 'none';
    runBtnText.textContent = 'Run Code';
    runBtn.disabled = false;

  } catch (error) {
    console.error('Python execution error:', error);
    outputContent.textContent = `Error communicating with server: ${error.message}`;
    outputImages.innerHTML = '';
    outputDiv.style.display = 'block';
    loadingDiv.style.display = 'none';
    runBtnText.textContent = 'Run Code';
    runBtn.disabled = false;
  }
}

function resetCode() {
  if (codeEditor) {
    const requestIdx = window.currentNotebookRequestIdx || 0;
    const defaultCode = `# Polytope Data Visualization - Request ${requestIdx + 1}
# The data is available in the 'polytope_data' variable

import json
import numpy as np
import covjsonkit
import earthkit.plots

from covjsonkit.api import Covjsonkit

decoder = Covjsonkit().decode(polytope_data)

ds = decoder.to_xarray()

print(ds)

# Handle missing/masked values
if '2t' in ds:
    data = ds['2t']
    # Replace NaN with a fill value or drop them
    data_filled = data.where(~np.isnan(data), drop=True)

    chart = earthkit.plots.Map(domain="Germany")
    chart.point_cloud(
        data_filled,
        x="longitude",
        y="latitude",
        auto_style=True
    )

    chart.coastlines()
    chart.borders()
    chart.gridlines()

    chart.title("{variable_name} (number={number})")

    chart.legend()
else:
    print("Variable '2t' not found in dataset")
    print("Available variables:", list(ds.data_vars))

# chart.show()  # Not needed - figure is captured automatically
`;
    codeEditor.setValue(defaultCode);
  }

  // Clear output
  const outputDiv = document.getElementById('notebook-output');
  outputDiv.style.display = 'none';
}

function closeNotebook() {
  const notebookSection = document.getElementById('notebook-section');
  notebookSection.style.display = 'none';

  // Clear output
  const outputDiv = document.getElementById('notebook-output');
  outputDiv.style.display = 'none';
}

// Expose initializeViewer globally so catalogue_wasm.js can call it once the
// WASM catalogue (or the server fallback) is ready.
window.initializeViewer = initializeViewer;

// Show a loading spinner — catalogue_wasm.js will replace this once ready.
const _itemsEl = document.getElementById("items");
if (_itemsEl) {
  _itemsEl.innerHTML = '<p style="padding:1rem;color:#888">⏳ Loading catalogue…</p>';
}

// Safety net: if catalogue_wasm.js hasn't triggered a render within 8s
// (e.g. the .wasm file is missing), fall back to the server-side endpoint.
setTimeout(() => {
  if (!window.__wasmCatalogue && !window.__viewerStarted) {
    console.warn("[wasm] Timed out waiting for WASM — falling back to server");
    const badge = document.getElementById("wasm-status");
    if (badge) { badge.textContent = "🌐 Server"; badge.style.background = "#cce5ff"; badge.style.color = "#004085"; }
    initializeViewer();
  }
}, 8000);

// Add event listener for copy button
document.addEventListener("DOMContentLoaded", () => {
  const copyBtn = document.getElementById("copy-mars-btn");
  if (copyBtn) {
    copyBtn.addEventListener("click", copyMARSRequests);
  }

  // Add event listener for Polytope button
  const polytopeBtn = document.getElementById('polytope-btn');
  if (polytopeBtn) {
    polytopeBtn.addEventListener('click', queryPolytope);
  }

  // Add event listener for close notebook button
  const closeNotebookBtn = document.getElementById('close-notebook-btn');
  if (closeNotebookBtn) {
    closeNotebookBtn.addEventListener('click', closeNotebook);
  }

  // Add event listener for run code button
  const runCodeBtn = document.getElementById('run-code-btn');
  if (runCodeBtn) {
    runCodeBtn.addEventListener('click', runPythonCode);
  }

  // Add event listener for reset code button
  const resetCodeBtn = document.getElementById('reset-code-btn');
  if (resetCodeBtn) {
    resetCodeBtn.addEventListener('click', resetCode);
  }
});