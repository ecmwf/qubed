parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$parent_path"
sudo LOCAL_CACHE=True ../../.venv/bin/fastapi dev ./main.py --port 80 --host=0.0.0.0 --reload
