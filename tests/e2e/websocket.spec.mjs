import { expect, test } from "@playwright/test";
import { login } from "./helpers/auth.mjs";

test("websocket endpoint responds to ping commands", async ({ page }) => {
    await login(page);

    const message = await page.evaluate(async () => {
        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const socket = new WebSocket(`${protocol}//${window.location.host}/ws`);

        return await new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                socket.close();
                reject(new Error("WebSocket timed out"));
            }, 3000);

            socket.addEventListener("open", () => {
                socket.send(JSON.stringify({ type: "ping" }));
            });

            socket.addEventListener("message", (event) => {
                clearTimeout(timeout);
                socket.close();
                resolve(event.data);
            });

            socket.addEventListener("error", () => {
                clearTimeout(timeout);
                reject(new Error("WebSocket connection failed"));
            });
        });
    });

    expect(message).toBe('{"type":"pong"}');
});
