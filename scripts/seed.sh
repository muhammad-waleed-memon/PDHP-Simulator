#!/bin/bash
# ==============================================================
# seed.sh — Populate demo data for development/demonstration
# ==============================================================
# This script inserts clearly labeled DEMO DATA into the database.
# It is NOT intended for production use.
# Run after: docker compose up
# ==============================================================

set -euo pipefail

echo "================================================"
echo "Pakistan Digital Health Platform — Demo Seeder"
echo "================================================"
echo ""
echo "WARNING: This script inserts DEMO DATA only."
echo "All data is synthetic. No real patient information."
echo ""

# This will be implemented in Phase 2 when the database schema exists.
# The actual seeding logic will use the backend's built-in seeder
# (triggered by SEED_DEMO_DATA=true environment variable).

echo "Demo data seeding is performed automatically by the backend"
echo "when APP_ENV=development and SEED_DEMO_DATA=true."
echo ""
echo "To trigger manually, restart the backend:"
echo "  docker compose restart backend"
