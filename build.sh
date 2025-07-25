#!/bin/bash

# Build script for maxwellpaulm.github.io
# Downloads latest resume and prepares site for deployment

set -e

echo "Building website..."

# Create assets directory if it doesn't exist
mkdir -p assets

# Download latest resume using GitHub API
echo "Fetching latest resume release..."

if [ -z "$GITHUB_TOKEN" ]; then
    echo "Error: GITHUB_TOKEN environment variable is required for private repo access"
    exit 1
fi

# Get asset API URL from the latest release
ASSET_URL=$(curl -s -H "Authorization: token $GITHUB_TOKEN" \
  https://api.github.com/repos/maxwellpaulm/resume/releases/latest | \
  grep -B5 "paul_maxwell_resume.pdf" | \
  grep '"url"' | head -1 | cut -d '"' -f 4)

if [ -z "$ASSET_URL" ]; then
    echo "Error: Could not find resume PDF in latest release"
    exit 1
fi

echo "Downloading from: $ASSET_URL"
curl -L -H "Authorization: token $GITHUB_TOKEN" \
  -H "Accept: application/octet-stream" \
  "$ASSET_URL" -o assets/paul_maxwell_resume.pdf

if [ $? -eq 0 ]; then
    echo "Resume downloaded successfully to assets/paul_maxwell_resume.pdf"
else
    echo "Error downloading resume"
    exit 1
fi

echo "Build complete!"
echo "Resume available at: assets/paul_maxwell_resume.pdf"