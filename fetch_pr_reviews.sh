#!/bin/bash

# Script to fetch GitHub Pull Request reviews for the current branch
# Usage: ./fetch_pr_reviews.sh

# Get the current branch name
CURRENT_BRANCH=$(git branch --show-current)

# List PRs for the current branch
echo "Fetching PRs for branch: $CURRENT_BRANCH"
gh pr list --head "$CURRENT_BRANCH"

# Get the PR number (assuming there's only one PR for this branch)
PR_NUMBER=$(gh pr list --head "$CURRENT_BRANCH" --json number --jq '.[0].number')

if [ -z "$PR_NUMBER" ]; then
    echo "No PR found for branch: $CURRENT_BRANCH"
    exit 1
fi

echo "Fetching reviews for PR #$PR_NUMBER"
gh pr view "$PR_NUMBER" --json reviews,comments
