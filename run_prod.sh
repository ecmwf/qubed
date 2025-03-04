cd backend
# sudo ../.venv/bin/fastapi dev main.py --port 80
sudo ../.venv/bin/uvicorn main:app --port 80 --host 0.0.0.0 --reload\
    --reload-include="*.html" \
    --reload-include="*.css" \
    --reload-include="*.js" \
    --reload-include="*.yaml"
