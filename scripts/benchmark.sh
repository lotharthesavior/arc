#!/bin/bash

# Arc Benchmark Script
# Uses wrk to benchmark the application
#
# Prerequisites:
#   - wrk must be installed (https://github.com/wg/wrk)
#   - The application must be running (cargo run develop)
#
# Usage: ./scripts/benchmark.sh [options]
#   Options:
#     -u, --url       Base URL (default: http://127.0.0.1:8080)
#     -t, --threads   Number of threads (default: 4)
#     -c, --connections Number of connections (default: 100)
#     -d, --duration  Duration of test (default: 30s)
#     -h, --help      Show this help message

set -e

# Read port from .env file if it exists
PORT=8080  # Default port
if [ -f ".env" ]; then
    # Extract APP_PORT from .env file
    ENV_PORT=$(grep -E "^APP_PORT=" .env | cut -d '=' -f2)
    if [ ! -z "$ENV_PORT" ]; then
        PORT=$ENV_PORT
    fi
fi

# Default values
BASE_URL="http://127.0.0.1:${PORT}"
THREADS=4
CONNECTIONS=100
DURATION="30s"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -u|--url)
            BASE_URL="$2"
            shift 2
            ;;
        -t|--threads)
            THREADS="$2"
            shift 2
            ;;
        -c|--connections)
            CONNECTIONS="$2"
            shift 2
            ;;
        -d|--duration)
            DURATION="$2"
            shift 2
            ;;
        -h|--help)
            echo "Arc Benchmark Script"
            echo ""
            echo "Usage: ./scripts/benchmark.sh [options]"
            echo ""
            echo "Options:"
            echo "  -u, --url         Base URL (default: http://127.0.0.1:[PORT from .env or 8080])"
            echo "  -t, --threads     Number of threads (default: 4)"
            echo "  -c, --connections Number of connections (default: 100)"
            echo "  -d, --duration    Duration of test (default: 30s)"
            echo "  -h, --help        Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if wrk is installed
if ! command -v wrk &> /dev/null; then
    echo "Error: wrk is not installed."
    echo ""
    echo "Install wrk:"
    echo "  Arch Linux: sudo pacman -S wrk"
    echo "  Ubuntu/Debian: sudo apt install wrk"
    echo "  macOS: brew install wrk"
    echo "  Or build from source: https://github.com/wg/wrk"
    exit 1
fi

# Check if the server is running
if ! curl -s -o /dev/null -w "%{http_code}" "$BASE_URL" | grep -q "200\|302"; then
    echo "Error: Server does not appear to be running at $BASE_URL"
    echo "Start the server with: cargo run develop"
    exit 1
fi

echo "=============================================="
echo "  Arc Benchmark"
echo "=============================================="
echo ""
echo "Configuration:"
echo "  Base URL:    $BASE_URL"
echo "  Threads:     $THREADS"
echo "  Connections: $CONNECTIONS"
echo "  Duration:    $DURATION"
echo ""

# Benchmark the sample for comparison
echo "----------------------------------------------"
echo "Benchmarking: GET / (Home Page)"
echo "----------------------------------------------"
wrk -t"$THREADS" -c"$CONNECTIONS" -d"$DURATION" "http://localhost:8082/"
echo ""


# Benchmark the home page
echo "----------------------------------------------"
echo "Benchmarking: GET / (Home Page)"
echo "----------------------------------------------"
wrk -t"$THREADS" -c"$CONNECTIONS" -d"$DURATION" "$BASE_URL/"
echo ""

# Benchmark the signin page
echo "----------------------------------------------"
echo "Benchmarking: GET /signin (Sign In Page)"
echo "----------------------------------------------"
wrk -t"$THREADS" -c"$CONNECTIONS" -d"$DURATION" "$BASE_URL/signin"
echo ""

# Benchmark static assets
echo "----------------------------------------------"
echo "Benchmarking: GET /public/imgs/arc-logo.png (Static Asset)"
echo "----------------------------------------------"
wrk -t"$THREADS" -c"$CONNECTIONS" -d"$DURATION" "$BASE_URL/public/imgs/arc-logo.png"
echo ""

echo "=============================================="
echo "  Benchmark Complete"
echo "=============================================="
