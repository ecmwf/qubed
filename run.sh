cd backend
# ../.venv/bin/fastapi dev main.py
../.venv/bin/uvicorn main:app --reload \
    --reload-include="*.html" \
    --reload-include="*.css" \
    --reload-include="*.js" \
    --reload-include="*.yaml"
