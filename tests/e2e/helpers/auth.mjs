import { expect } from "@playwright/test";

export async function login(page) {
    await page.goto("/signin");
    await page.getByLabel("Email address").fill("jekyll@example.com");
    await page.getByLabel("Password").fill("password");
    await page.getByRole("button", { name: "Sign in" }).click();

    await expect(page).toHaveURL(/\/admin$/);
}
