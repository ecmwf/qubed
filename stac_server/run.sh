parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"
REDIS_HOST=localhost CONFIG_DIR=../config/destinE fastapi dev ./main.py --port 8124 --reload