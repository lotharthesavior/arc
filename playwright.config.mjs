import { defineConfig } from "@playwright/test";

const secretKey =
    "f3782qghf784rohgf784royhfv894hfdfnmwuiasfhreiuohiuwerj4f3897qw-0pjfi4ro";
const jwtSecret = "test-jwt-secret-at-least-32-characters-long";
const serverCommand = [
    "rm -f database/e2e.sqlite",
    `export APP_URL=127.0.0.1 APP_PORT=8082 DATABASE_URL=database/e2e.sqlite SECRET_KEY=${secretKey} ENABLE_JWT_AUTH=true JWT_SECRET=${jwtSecret} JWT_EXPIRY_HOURS=24`,
    "cargo run migrate",
    "cargo run seed",
    "npm run build",
    "cargo run serve",
].join(" && ");

export default defineConfig({
    testDir: "./tests/e2e",
    timeout: 30_000,
    fullyParallel: false,
    workers: 1,
    use: {
        baseURL: "http://127.0.0.1:8082",
        browserName: "chromium",
        headless: true,
        launchOptions: {
            executablePath: "/usr/bin/chromium",
            args: ["--no-sandbox"],
        },
    },
    webServer: {
        command: `bash -lc '${serverCommand}'`,
        url: "http://127.0.0.1:8082/signin",
        reuseExistingServer: false,
        timeout: 120_000,
    },
});
