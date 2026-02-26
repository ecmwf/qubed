# Polytope Authentication

The STAC server's Polytope integration collects authentication credentials directly from users through the web interface and queries the **Destination Earth** Polytope service.

## User Flow

1. Navigate through the STAC catalogue and select your data
2. At the end of the catalogue, you'll see the "Query Data with Polytope" section
3. Enter your Destination Earth Polytope credentials:
   - **Email Address**: Your email address registered with Destination Earth
   - **API Key**: Your Polytope API key
4. Click "Query Polytope Service" to submit your data extraction requests

## Getting Your Polytope API Key

1. Visit [https://polytope.lumi.apps.dte.destination-earth.eu](https://polytope.lumi.apps.dte.destination-earth.eu)
2. Log in with your Destination Earth credentials
3. Navigate to your profile or settings
4. Generate or copy your API key

## Technical Details

The service connects to the Destination Earth Polytope instance:
- **Address**: `polytope.lumi.apps.dte.destination-earth.eu`
- **Collection**: `destination-earth`

## Security Notes

- Credentials are sent securely with each request
- Credentials are not stored on the server
- Each user provides their own authentication
- Credentials are only logged in a masked format for debugging

## For Developers

The credentials are sent in the request body as:
```json
{
  "requests": [...],
  "credentials": {
    "user_email": "user@ecmwf.int",
    "user_key": "your_api_key"
  }
}
```

The backend passes these credentials directly to `earthkit.data.from_source()` when querying Polytope.
