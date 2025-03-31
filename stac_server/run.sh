parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"
LOCAL_CACHE=True fastapi dev ./main.py --port 8124 --reload
