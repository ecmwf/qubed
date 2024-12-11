parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"
CONFIG_DIR=../config/local fastapi dev ./main.py --port 8124 --reload