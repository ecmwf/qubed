#!/usr/bin/env bash
cd "$(dirname "$0")"
sphinx-build --define nb_execution_raise_on_error=1 . _build
