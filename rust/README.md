# Deploy Manager RS

A Rust-based GitHub webhook server for automated deployments using Rocket.rs. This server receives GitHub webhook events and triggers Docker Compose deployments with Discord notifications.

## Features

- **GitHub Webhook Integration**: Receives and validates GitHub push events
- **HMAC Signature Verification**: Ensures webhooks are authentic using repository-specific secrets
- **Branch-specific Deployments**: Optional branch filtering for deployments
- **Discord Notifications**: Success/failure notifications via Discord webhooks
- **Admin API**: REST API for managing repository configurations
- **Docker Compose Support**: Automated deployment using Docker Compose

## Installation

1. Clone the repository:
```bash
git clone <your-repo-url>
cd deploy-manager-rs
```

2. Install Rust (if not already installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

3. Build the project:
```bash
cargo build --release
```

## Configuration

1. Copy the example environment file:
```bash
cp .env.example .env
```

2. Edit `.env` with your configuration:
```env
PORT=9786
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/YOUR_WEBHOOK_URL_HERE
ADMIN_TOKEN=your-secure-admin-token-here
```

## Running the Server

```bash
cargo run --release
```

Or run the binary directly:
```bash
./target/release/deploy-manager-rs
```

## API Endpoints

### Webhook Endpoint
- **POST** `/webhook` - Receives GitHub webhook events

### Admin Endpoints (require `x-admin-token` header)

#### List Repositories
```bash
curl -H "x-admin-token: your-admin-token" \
     http://localhost:9786/admin/repos
```

#### Add/Update Repository
```bash
curl -X POST \
     -H "Content-Type: application/json" \
     -H "x-admin-token: your-admin-token" \
     -d '{
       "repoName": "username/repository-name",
       "path": "/path/to/local/repository",
       "secret": "github-webhook-secret",
       "branch": "main"
     }' \
     http://localhost:9786/admin/repos
```

#### Delete Repository
```bash
curl -X DELETE \
     -H "Content-Type: application/json" \
     -H "x-admin-token: your-admin-token" \
     -d '{"repoName": "username/repository-name"}' \
     http://localhost:9786/admin/repos
```

## GitHub Webhook Setup

1. Go to your GitHub repository settings
2. Navigate to "Webhooks" and click "Add webhook"
3. Set the payload URL to: `http://your-server:9786/webhook`
4. Set content type to: `application/json`
5. Enter the webhook secret (must match the `secret` in repository configuration)
6. Select "Just the push event"
7. Ensure the webhook is active

## Repository Configuration

Each repository must be configured with:

- **repoName**: GitHub repository name in format `username/repository-name`
- **path**: Local path where the repository is cloned
- **secret**: GitHub webhook secret for HMAC verification
- **branch**: (Optional) Specific branch to deploy (if not set, all pushes trigger deployment)

## Deployment Process

When a valid webhook is received, the server executes:

1. `cd /path/to/repository`
2. `git pull`
3. `cd ..`
4. `docker compose up -d --build`
5. Verification check to ensure containers are running

## Discord Notifications

If `DISCORD_WEBHOOK_URL` is configured, the server sends notifications for:

- ✅ Successful deployments
- ❌ Failed deployments

Notifications include:
- Repository name
- Deployment path
- Timestamp
- Error details (for failures)

## Security

- All webhook requests are verified using HMAC-SHA256 signatures
- Admin endpoints require authentication via `x-admin-token` header
- Repository-specific secrets ensure webhook authenticity

## File Structure

```
src/
├── main.rs          # Application entry point and Rocket setup
├── admin.rs         # Admin API endpoints for repository management
├── webhook.rs       # GitHub webhook handler
├── deploy.rs        # Deployment execution and Discord notifications
├── config.rs        # Configuration file management
└── types.rs         # Data structures and types
```

## Configuration Storage

Repository configurations are stored in `repo-config.json` in the current working directory. This file is automatically created and managed by the application.

## Example Docker Compose Setup

Your repository should contain a `docker-compose.yml` file for deployment:

```yaml
version: '3.8'
services:
  app:
    build: .
    ports:
      - "3000:3000"
    restart: unless-stopped
```

## Troubleshooting

### Common Issues

1. **Webhook not triggering deployments**: Check GitHub webhook delivery logs and ensure the secret matches
2. **Authentication failures**: Verify the `x-admin-token` header matches the `ADMIN_TOKEN` environment variable
3. **Deployment failures**: Check Docker Compose configuration and ensure the repository path is correct

### Logs

The application logs to stdout/stderr. Key information includes:
- Webhook receipt and validation status
- Deployment command execution
- Discord notification status
- Configuration changes

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License.
