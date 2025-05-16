import * as http from "http";
import * as crypto from "crypto";
import { exec } from "child_process";
import * as path from "path";
import * as fs from "fs";
import * as https from "https";
import * as dotenv from "dotenv";

dotenv.config();

// Type definitions
interface RepoConfig {
    path: string;
    secret: string;
    branch: string | null;
}

interface RepoConfigs {
    [key: string]: RepoConfig;
}

interface WebhookPayload {
    repository?: {
        full_name: string;
    };
    ref?: string;
}

interface NotificationPayload {
    text: string;
}

// Repository configurations
const initialRepoConfigs: RepoConfigs = {};

// Global configuration
const PORT: number = 9786;
const DISCORD_WEBHOOK: string = process.env.DISCORD_WEBHOOK_URL ?? "";

function handleAdminRequest(
    req: http.IncomingMessage,
    res: http.ServerResponse
): void {
    if (req.method === "GET") {
        // List all configured repositories
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify(repoConfigs));
        return;
    }

    if (req.method === "POST" || req.method === "PUT") {
        // Add or update repository configuration
        let body: string = "";
        req.on("data", (chunk: Buffer) => {
            body += chunk.toString();
        });

        req.on("end", () => {
            try {
                const newConfig: RepoConfig & { repoName: string } =
                    JSON.parse(body);

                // Validate required fields
                if (
                    !newConfig.repoName ||
                    !newConfig.path ||
                    !newConfig.secret
                ) {
                    res.statusCode = 400;
                    res.end(
                        JSON.stringify({
                            error: "Missing required fields: repoName, path, and secret are required",
                        })
                    );
                    return;
                }

                // Update configuration
                repoConfigs[newConfig.repoName] = {
                    path: newConfig.path,
                    secret: newConfig.secret,
                    branch: newConfig.branch || null,
                };

                // Save to file
                saveRepoConfigs(repoConfigs);

                res.writeHead(200, { "Content-Type": "application/json" });
                res.end(
                    JSON.stringify({
                        success: true,
                        message: `Repository ${newConfig.repoName} configured successfully`,
                    })
                );
            } catch (error) {
                res.statusCode = 400;
                res.end(
                    JSON.stringify({
                        error: `Invalid request: ${(error as Error).message}`,
                    })
                );
            }
        });
        return;
    }

    if (req.method === "DELETE") {
        // Remove repository configuration
        let body: string = "";
        req.on("data", (chunk: Buffer) => {
            body += chunk.toString();
        });

        req.on("end", () => {
            try {
                const { repoName }: { repoName: string } = JSON.parse(body);

                if (!repoName || !repoConfigs[repoName]) {
                    res.statusCode = 404;
                    res.end(
                        JSON.stringify({
                            error: `Repository ${repoName} not found`,
                        })
                    );
                    return;
                }

                // Remove from configuration
                delete repoConfigs[repoName];

                // Save to file
                saveRepoConfigs(repoConfigs);

                res.writeHead(200, { "Content-Type": "application/json" });
                res.end(
                    JSON.stringify({
                        success: true,
                        message: `Repository ${repoName} removed successfully`,
                    })
                );
            } catch (error) {
                res.statusCode = 400;
                res.end(
                    JSON.stringify({
                        error: `Invalid request: ${(error as Error).message}`,
                    })
                );
            }
        });
        return;
    }

    // Method not allowed
    res.statusCode = 405;
    res.end(JSON.stringify({ error: "Method not allowed" }));
}

const server = http.createServer(
    (req: http.IncomingMessage, res: http.ServerResponse) => {
        // Admin endpoint for managing repos (requires admin token)
        if (
            req.url?.startsWith("/admin/repos") &&
            req.headers["x-admin-token"] === process.env.ADMIN_TOKEN
        ) {
            handleAdminRequest(req, res);
            return;
        }

        // Only respond to POST requests on the webhook endpoint
        if (req.method !== "POST" || req.url !== "/webhook") {
            res.statusCode = 404;
            res.end("Not Found");
            return;
        }

        let body: string = "";
        req.on("data", (chunk: Buffer) => {
            body += chunk.toString();
        });

        req.on("end", () => {
            let payload: WebhookPayload;
            try {
                payload = JSON.parse(body);
            } catch (e) {
                console.error("Invalid JSON payload");
                res.statusCode = 400;
                res.end("Bad Request: Invalid JSON");
                return;
            }

            // Extract repository information
            const fullRepoName = payload.repository?.full_name;
            if (!fullRepoName) {
                console.error("Repository name not found in payload");
                res.statusCode = 400;
                res.end("Bad Request: Repository not specified");
                return;
            }

            // Find matching repository configuration
            const repoConfig = repoConfigs[fullRepoName];
            if (!repoConfig) {
                console.error(
                    `No configuration found for repository: ${fullRepoName}`
                );
                res.statusCode = 404;
                res.end("Repository not configured");
                return;
            }

            // Verify the request is coming from GitHub
            const signature = req.headers["x-hub-signature-256"] as string;
            if (!signature) {
                console.error("No signature provided");
                res.statusCode = 401;
                res.end("Unauthorized");
                return;
            }

            // Calculate expected signature using repo-specific secret
            const hmac = crypto.createHmac("sha256", repoConfig.secret);
            const expectedSignature =
                "sha256=" + hmac.update(body).digest("hex");

            // Verify signatures match
            if (signature !== expectedSignature) {
                console.error("Invalid signature");
                res.statusCode = 401;
                res.end("Unauthorized");
                return;
            }

            // Check if this is a push event
            const event = req.headers["x-github-event"] as string;
            if (event !== "push") {
                console.log(`Ignoring event: ${event}`);
                res.statusCode = 200;
                res.end("OK");
                return;
            }

            // Check if this is the configured branch (if specified)
            if (repoConfig.branch) {
                const branch = payload.ref?.replace("refs/heads/", "");
                if (branch !== repoConfig.branch) {
                    console.log(
                        `Ignoring push to branch ${branch}, only deploying ${repoConfig.branch}`
                    );
                    res.statusCode = 200;
                    res.end(`OK: Ignored push to ${branch}`);
                    return;
                }
            }

            console.log(
                `Received valid webhook for ${fullRepoName}, deploying...`
            );

            deployRepository(repoConfig, fullRepoName, res);
        });
    }
);

function deployRepository(
    repoConfig: RepoConfig,
    repoName: string,
    res: http.ServerResponse
): void {
    const repoPath = repoConfig.path;

    // Execute deployment script with checks and versioning
    const deploymentVersion = Date.now();
    const commands = [
        `cd ${repoPath}`,
        "git pull",
        // Pre-deployment checks
        'test -f docker-compose.yml || (echo "docker-compose.yml not found" && exit 1)',
        // Validate docker-compose file
        'docker compose config -q || (echo "Invalid docker-compose.yml" && exit 1)',
        // Perform the zero-downtime update
        `DEPLOYMENT_VERSION=${deploymentVersion} docker compose up -d --build`,
        // Verify deployment success
        'sleep 10 && docker compose ps --filter "status=running" | grep -v "Down" || (echo "Deployment verification failed" && exit 1)',
    ].join(" && ");

    console.log(`Executing deployment commands for ${repoName}: ${commands}`);

    exec(commands, (error, stdout, stderr) => {
        if (error) {
            console.error(`Deployment error for ${repoName}: ${error.message}`);

            if (DISCORD_WEBHOOK) {
                const url = new URL(DISCORD_WEBHOOK);

                const notification: NotificationPayload = {
                    text: `❌ Deployment failed for ${repoName}\nPath: ${repoConfig.path}\nError: ${error.message}`,
                };

                const requestData = JSON.stringify(notification);

                const req = https.request({
                    hostname: url.hostname,
                    path: url.pathname + url.search,
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json",
                        "Content-Length": Buffer.byteLength(requestData),
                    },
                });

                req.on("error", (error) => {
                    console.error(
                        `Error sending Discord notification: ${error.message}`
                    );
                });

                req.write(requestData);
                req.end();
            }

            return;
        }

        console.log(`Deployment successful for ${repoName}`);
        console.log("Output:", stdout);

        if (stderr) {
            console.error("Errors:", stderr);
        }

        // Send notification if configured
        if (DISCORD_WEBHOOK) {
            const url = new URL(DISCORD_WEBHOOK);

            const notification = {
                content: `✅ Deployment successful for ${repoName}\nPath: ${
                    repoConfig.path
                }\nTimestamp: ${new Date().toISOString()}`,
            };

            const requestData = JSON.stringify(notification);

            const req = https.request({
                hostname: url.hostname,
                path: url.pathname + url.search,
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    "Content-Length": Buffer.byteLength(requestData),
                },
            });

            req.on("error", (error) => {
                console.error(
                    `Error sending Discord notification: ${error.message}`
                );
            });

            req.write(requestData);
            req.end();
        }

        res.statusCode = 200;
        res.end(`Deployment successful for ${repoName}`);
    });
}

// Utilities for managing repositories
function loadRepoConfigs(): RepoConfigs {
    const configPath = path.join(__dirname, "repo-config.json");
    try {
        if (fs.existsSync(configPath)) {
            const configData = fs.readFileSync(configPath, "utf8");
            return JSON.parse(configData);
        }
    } catch (error) {
        console.error(
            `Error loading repo configurations: ${(error as Error).message}`
        );
    }

    // Return default if file doesn't exist or has errors
    return initialRepoConfigs;
}

function saveRepoConfigs(configs: RepoConfigs): boolean {
    const configPath = path.join(__dirname, "repo-config.json");
    try {
        fs.writeFileSync(configPath, JSON.stringify(configs, null, 2), "utf8");
        return true;
    } catch (error) {
        console.error(
            `Error saving repo configurations: ${(error as Error).message}`
        );
        return false;
    }
}

// Load configurations from file at startup
let repoConfigs: RepoConfigs = loadRepoConfigs();

server.listen(PORT, "0.0.0.0", () => {
    console.log(`Webhook server listening on port ${PORT}`);
    console.log(
        `Configured repositories: ${Object.keys(repoConfigs).join(", ")}`
    );
});
