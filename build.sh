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
echo $GITHUB_TOKEN

# Get latest release and extract download URL
LATEST_URL=$(curl -s -H "Authorization: token $GITHUB_TOKEN" \
  https://api.github.com/repos/maxwellpaulm/resume/releases/latest | \
  grep "browser_download_url.*resume.pdf" | \
  cut -d '"' -f 4)

if [ -z "$LATEST_URL" ]; then
    echo "Error: Could not find resume PDF in latest release"
    exit 1
fi

echo "Downloading from: $LATEST_URL"
curl -L -H "Authorization: token $GITHUB_TOKEN" \
  "$LATEST_URL" -o assets/paul_maxwell_resume.pdf

if [ $? -eq 0 ]; then
    echo "Resume downloaded successfully to assets/paul_maxwell_resume.pdf"
else
    echo "Error downloading resume"
    exit 1
fi

echo "Build complete!"
echo "Resume available at: assets/paul_maxwell_resume.pdf"