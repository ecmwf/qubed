parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"
API_KEY=asdf LOCAL_CACHE=True uv run fastapi dev ./main.py --port 8124 --reload
