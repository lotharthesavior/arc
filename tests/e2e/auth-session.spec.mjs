import { expect, test } from "@playwright/test";
import { login } from "./helpers/auth.mjs";

test("session auth redirects into admin and websocket controller connects", async ({
    page,
}) => {
    await page.addInitScript(() => {
        window.__wsStatuses = [];
        window.addEventListener("websocket:status", (event) => {
            window.__wsStatuses.push(event.detail.status);
        });
    });

    await page.goto("/admin");
    await expect(page).toHaveURL(/\/signin$/);

    await login(page);

    await expect(page.getByRole("main")).toContainText("Dashboard");
    await expect
        .poll(() => page.evaluate(() => window.__wsStatuses))
        .toContain("connected");
});
