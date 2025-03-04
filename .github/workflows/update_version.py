import re
import subprocess
from pathlib import Path

CARGO_TOML_PATH = Path("Cargo.toml")


# Get the latest Git tag and strip the leading 'v' if present
def get_git_version():
    try:
        version = subprocess.check_output(
            ["git", "describe", "--tags", "--always"], text=True
        ).strip()
        version = re.sub(r"^v", "", version)  # Remove leading 'v'
        return version
    except subprocess.CalledProcessError:
        raise RuntimeError(
            "Failed to get Git tag. Make sure you have at least one tag in the repository."
        )


# Update version in Cargo.toml
def update_cargo_version(new_version):
    cargo_toml = CARGO_TOML_PATH.read_text()

    # Replace version in [package] section
    updated_toml = re.sub(
        r'^version = "[^"]+"',
        f'version = "{new_version}"',
        cargo_toml,
        flags=re.MULTILINE,
    )

    CARGO_TOML_PATH.write_text(updated_toml)


if __name__ == "__main__":
    version = get_git_version()
    print(f"Parsed version: {version}")

    update_cargo_version(version)
    print(f"Updated Cargo.toml with version: {version}")
