parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"
LOCAL_CACHE=../config/climate-dt fastapi dev ./main.py --port 8124 --reload
