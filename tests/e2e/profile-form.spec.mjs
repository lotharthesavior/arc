import { expect, test } from "@playwright/test";
import { login } from "./helpers/auth.mjs";

test("profile form controller submits with fetch and updates the form", async ({
    page,
}) => {
    await login(page);
    await page.goto("/admin/profile");

    await page.evaluate(() => {
        const originalFetch = window.fetch.bind(window);
        window.__profileRequestBody = null;

        window.fetch = async (url, options) => {
            const target = typeof url === "string" ? url : url.url;

            if (target.endsWith("/admin/profile")) {
                window.__profileRequestBody = options?.body?.toString() ?? null;

                return new Response(
                    JSON.stringify({
                        data: {
                            name: "Updated Tester",
                            email: "updated@example.com",
                        },
                    }),
                    {
                        status: 200,
                        headers: { "Content-Type": "application/json" },
                    }
                );
            }

            return originalFetch(url, options);
        };
    });

    await page.locator("#name").fill("Updated Tester");
    await page.locator("#email").fill("updated@example.com");
    await page.locator("#profile-form button[type='submit']").click();

    await expect(page.locator(".toastify")).toContainText(
        "Profile updated successfully"
    );
    await expect(page.locator("#name")).toHaveValue("Updated Tester");
    await expect(page.locator("#email")).toHaveValue("updated@example.com");

    const requestBody = await page.evaluate(() => window.__profileRequestBody);
    expect(requestBody).toContain("name=Updated+Tester");
    expect(requestBody).toContain("email=updated%40example.com");
});
