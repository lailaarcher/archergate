#!/bin/bash
set -e

echo "Starting Archergate Minimal Example"
echo "===================================="
echo ""

# Ensure docker-compose is running in background
echo "1. Starting Archergate License Server..."
docker-compose up -d
echo "   Waiting for server to be healthy..."
sleep 10

# Grab the server URL
SERVER="http://localhost:3000"

echo ""
echo "2. Creating test API key..."
API_KEY=$(curl -s -X POST "$SERVER/keys" \
  -H "Content-Type: application/json" \
  -d '{"name":"test-api-key"}' | jq -r '.key')
echo "   API Key: $API_KEY"

echo ""
echo "3. Creating test license..."
LICENSE_KEY=$(curl -s -X POST "$SERVER/licenses" \
  -H "Content-Type: application/json" \
  -d "{\"api_key\":\"$API_KEY\",\"type\":\"standard\"}" | jq -r '.key')
echo "   License Key: $LICENSE_KEY"

echo ""
echo "4. Running minimal client..."
echo "   (This will validate the license against the running server)"
echo ""

# Update the test key in the source before running
sed -i.bak "s/TEST-AAAA-BBBB-CCCC-DDDD/$LICENSE_KEY/" examples/minimal/src/main.rs

cd examples/minimal
cargo run

# Restore original test key
sed -i.bak "s/$LICENSE_KEY/TEST-AAAA-BBBB-CCCC-DDDD/" src/main.rs

echo ""
echo "5. Cleaning up..."
cd ../..
docker-compose down

echo ""
echo "Done! The SDK and server work end-to-end."
