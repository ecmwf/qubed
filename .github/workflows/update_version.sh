VERSION=$(git describe --tags --always | awk '{gsub(/^v/, ""); print}')
perl -pi -e "s/^version = \"[^\"]+\"/version = \"$VERSION\"/" Cargo.toml
