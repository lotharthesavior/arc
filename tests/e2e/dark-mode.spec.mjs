import { expect, test } from "@playwright/test";

test("dark mode toggle persists across reloads", async ({ page }) => {
    await page.goto("/signin");

    const html = page.locator("html");
    const toggle = page.getByRole("button", { name: "Toggle dark mode" });

    await toggle.click();
    await expect(html).toHaveClass(/dark/);

    await page.reload();
    await expect(html).toHaveClass(/dark/);
});
