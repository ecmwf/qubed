# Qubed API Documentation

This book documents the available Rust and Python APIs for the Qubed project.

Build and serve the book locally:

```bash
# install mdbook if needed
cargo install mdbook

# build
mdbook build docs/book

# serve locally
mdbook serve -o docs/book
```

Rust API reference generation (optional):

```bash
# generate rustdoc
cargo doc --workspace --open
```

Python extension build (for Python API live-testing):

```bash
# from repository root
cd py_qubed
maturin develop --release
cd ../py_qubed_meteo
maturin develop --release
```
