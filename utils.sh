#!/bin/bash

# Configuration
WEBHOOK_SERVER="http://localhost:9786"

# Help message
show_help() {
    echo "Usage: $0 [add|list|remove] [options]"
    echo
    echo "Commands:"
    echo "  add    Add or update a repository configuration"
    echo "  list   List all configured repositories"
    echo "  remove Remove a repository configuration"
    echo
    echo "Options for 'add':"
    echo "  -n, --name    Repository name in format 'owner/repo'"
    echo "  -p, --path    Full path to repository on the server"
    echo "  -s, --secret  GitHub webhook secret"
    echo "  -b, --branch  Optional: branch to deploy (default: all branches)"
    echo
    echo "Options for 'remove':"
    echo "  -n, --name    Repository name to remove"
    echo
    echo "Example:"
    echo "  $0 add -n owner/my-repo -p /var/www/my-repo -s mysecret -b main"
}

# List repositories
list_repos() {
    echo "Fetching repository configurations..."
    curl -s -H "X-Admin-Token: $ADMIN_TOKEN" "$WEBHOOK_SERVER/admin/repos" | jq .
}

# Add repository
add_repo() {
    local repo_name=""
    local repo_path=""
    local repo_secret=""
    local repo_branch=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
        -n | --name)
            repo_name="$2"
            shift 2
            ;;
        -p | --path)
            repo_path="$2"
            shift 2
            ;;
        -s | --secret)
            repo_secret="$2"
            shift 2
            ;;
        -b | --branch)
            repo_branch="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
        esac
    done

    # Validate required fields
    if [[ -z "$repo_name" || -z "$repo_path" || -z "$repo_secret" ]]; then
        echo "Error: Missing required fields"
        show_help
        exit 1
    fi

    # Create JSON payload
    local payload="{\"repoName\":\"$repo_name\",\"path\":\"$repo_path\",\"secret\":\"$repo_secret\""
    if [[ -n "$repo_branch" ]]; then
        payload="$payload,\"branch\":\"$repo_branch\""
    fi
    payload="$payload}"

    # Send request
    echo "Configuring repository $repo_name..."
    curl -s -X POST \
        -H "Content-Type: application/json" \
        -H "X-Admin-Token: $ADMIN_TOKEN" \
        -d "$payload" \
        "$WEBHOOK_SERVER/admin/repos" | jq .
}

# Remove repository
remove_repo() {
    local repo_name=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
        -n | --name)
            repo_name="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
        esac
    done

    # Validate required fields
    if [[ -z "$repo_name" ]]; then
        echo "Error: Repository name is required"
        show_help
        exit 1
    fi

    # Send request
    echo "Removing repository $repo_name..."
    curl -s -X DELETE \
        -H "Content-Type: application/json" \
        -H "X-Admin-Token: $ADMIN_TOKEN" \
        -d "{\"repoName\":\"$repo_name\"}" \
        "$WEBHOOK_SERVER/admin/repos" | jq .
}

# Main script
if [[ $# -lt 1 ]]; then
    show_help
    exit 1
fi

case "$1" in
add)
    shift
    add_repo "$@"
    ;;
list)
    list_repos
    ;;
remove)
    shift
    remove_repo "$@"
    ;;
help | -h | --help)
    show_help
    ;;
*)
    echo "Unknown command: $1"
    show_help
    exit 1
    ;;
esac
