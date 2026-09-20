#!/bin/bash
# ==============================================================
# reset-db.sh — Reset the development database
# ==============================================================
# WARNING: This DELETES all data and recreates from scratch.
# Only use in development.
# ==============================================================

set -euo pipefail

echo "============================================"
echo "WARNING: This will DELETE all database data!"
echo "============================================"
read -p "Are you sure? Type 'yes' to continue: " confirm

if [ "$confirm" != "yes" ]; then
    echo "Aborted."
    exit 0
fi

echo "Stopping services..."
docker compose down -v --remove-orphans

echo "Rebuilding..."
docker compose up --build -d

echo "Database reset complete."
echo "Demo data will be seeded automatically on backend startup."
