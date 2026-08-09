import assert from "node:assert/strict";
import { chromium } from "@playwright/test";

const baseUrl = process.argv[2];
assert(baseUrl, "usage: node scripts/check-generated-auth-ui.mjs <base-url>");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto(`${baseUrl}/admin`);
  await page.waitForURL(`${baseUrl}/signin`);
  const email = page.getByLabel("Email");
  const password = page.getByLabel("Password");
  await assert.doesNotReject(() => email.waitFor());
  await assert.doesNotReject(() => password.waitFor());

  const fieldLayout = await email.evaluate((input) => ({
    width: input.getBoundingClientRect().width,
    parentWidth: input.parentElement.getBoundingClientRect().width,
    labelDisplay: getComputedStyle(input.labels[0]).display,
  }));
  assert.equal(fieldLayout.labelDisplay, "block");
  assert(fieldLayout.width >= fieldLayout.parentWidth - 2, "email input must fill its field");

  await email.fill("admin@example.com");
  await password.fill("change-me-now");
  await page.getByRole("button", { name: "Sign in" }).click();
  await page.waitForURL(`${baseUrl}/admin`);
  await page.goto(`${baseUrl}/profile`);
  await page.getByLabel("Name").fill("Scaffold Administrator Updated");
  await page.getByRole("button", { name: "Save profile" }).click();
  await page.waitForURL(`${baseUrl}/profile`);
  assert.equal(await page.getByLabel("Name").inputValue(), "Scaffold Administrator Updated");
  await page.goto(`${baseUrl}/admin/users`);
  await page.getByRole("heading", { name: "Users" }).waitFor();
  await page.getByLabel("Roles").first().waitFor();
  await page.getByLabel("Name").fill("Second Operator");
  await page.getByLabel("Email").fill("second@example.com");
  await page.getByLabel("Temporary password").fill("second-password");
  await page.getByRole("button", { name: "Create user" }).click();
  await page.waitForURL(`${baseUrl}/admin/users`);
  await page.getByText("second@example.com").waitFor();
  assert.equal(errors.length, 0, errors.join("\n"));
} finally {
  await browser.close();
}
