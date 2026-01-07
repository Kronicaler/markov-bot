docker compose down db &&
Remove-Item -Recurse -Force ./data/postgres &&
docker compose up db -d