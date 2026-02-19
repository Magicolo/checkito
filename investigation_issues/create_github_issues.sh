#!/bin/bash

# Script to create GitHub issues from investigation findings
# This script uses the GitHub CLI (gh) to create issues from markdown files
#
# Prerequisites:
# 1. Install GitHub CLI: https://cli.github.com/
# 2. Authenticate: gh auth login
# 3. Run from repository root: ./investigation_issues/create_github_issues.sh

set -e

REPO="Magicolo/checkito"
ISSUES_DIR="$(dirname "$0")"

echo "Creating GitHub issues for checkito investigation findings..."
echo "Repository: $REPO"
echo "Issues directory: $ISSUES_DIR"
echo ""

# Check if gh is installed
if ! command -v gh &> /dev/null; then
    echo "ERROR: GitHub CLI (gh) is not installed"
    echo "Please install from: https://cli.github.com/"
    exit 1
fi

# Check if authenticated
if ! gh auth status &> /dev/null; then
    echo "ERROR: Not authenticated with GitHub CLI"
    echo "Please run: gh auth login"
    exit 1
fi

# Function to create issue from markdown file
create_issue() {
    local file="$1"
    local title=$(head -n 1 "$file" | sed 's/^# //')
    local body=$(tail -n +2 "$file")
    
    echo "Creating issue: $title"
    
    gh issue create \
        --repo "$REPO" \
        --title "$title" \
        --body "$body"
    
    echo "✓ Created: $title"
    echo ""
}

# Create issues from all markdown files (except INDEX)
for file in "$ISSUES_DIR"/*.md; do
    filename=$(basename "$file")
    
    # Skip the index file
    if [[ "$filename" == "00-INDEX.md" ]] || [[ "$filename" == "README.md" ]]; then
        continue
    fi
    
    create_issue "$file"
    
    # Rate limiting: wait 2 seconds between requests
    sleep 2
done

echo "All issues created successfully!"
echo "View issues at: https://github.com/$REPO/issues"
