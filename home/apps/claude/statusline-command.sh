#!/bin/bash

# Claude Code statusline script
# Receives JSON input via stdin and generates a status line

# Read JSON input
input=$(cat)

# Extract values from JSON
current_dir=$(echo "$input" | jq -r '.current_dir // .workspace.current_dir // ""')
project_dir=$(echo "$input" | jq -r '.project_dir // .workspace.project_dir // ""')
model_display=$(echo "$input" | jq -r '.model.display_name // ""')

# Calculate directory display
dir_display=""
if [ -n "$current_dir" ]; then
    if [ "$current_dir" = "$project_dir" ]; then
        dir_display=$(basename "$current_dir")
    elif [[ "$current_dir" == "$project_dir"/* ]]; then
        dir_display=$(echo "$current_dir" | sed "s|^$project_dir/||")
    else
        dir_display=$(basename "$current_dir")
    fi
fi

# Get git branch if in a git repo
git_branch=""
if [ -n "$current_dir" ] && cd "$current_dir" 2>/dev/null; then
    git_branch=$(git branch --show-current 2>/dev/null | head -1)

    # Truncate branch name if longer than 40 characters
    if [ -n "$git_branch" ] && [ ${#git_branch} -gt 40 ]; then
        # Show first 30 chars + "..." + last 7 chars
        git_branch="${git_branch:0:30}...${git_branch: -7}"
    fi
fi

# Build status line components
components=""

# User@host in colors
components="\033[95mpeki\033[97m@\033[96mgiskard\033[0m"

# Directory
if [ -n "$dir_display" ]; then
    components="$components \033[90m•\033[0m \033[94m$dir_display\033[0m"
fi

# Git branch
if [ -n "$git_branch" ]; then
    components="$components \033[90m•\033[0m \033[93m$git_branch\033[0m"
fi

# Model name
if [ -n "$model_display" ]; then
    components="$components \033[90m•\033[0m \033[92m$model_display\033[0m"
fi

# Output the complete status line
echo -e "$components"