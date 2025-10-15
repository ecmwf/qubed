# Qubed Catalogue Chart

This is helm chart for the frontend in `../stac_server`.

The default values are for our dev deployment.

It is meant to be deployed by skaffold from the parent directory.

Known issues:
- The cronjobs need to be run on the same node as the stac server to avoid pvc attachment issues.
They currently are not, so we don't suggest using them.