VERSION=$(git describe --tags --always | sed 's/^v//')
sed -i -E 's/^version = "[^"]+"/version = "'"$VERSION"'"/' Cargo.toml