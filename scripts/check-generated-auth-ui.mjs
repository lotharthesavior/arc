import assert from "node:assert/strict";
import { chromium } from "@playwright/test";

const baseUrl = process.argv[2];
assert(baseUrl, "usage: node scripts/check-generated-auth-ui.mjs <base-url>");

async function assertInset(page, outerSelector, innerSelector, label) {
  const inset = await page.locator(innerSelector).first().evaluate((inner, outerSelector) => {
    const paddingLeft = Number.parseFloat(getComputedStyle(inner).paddingLeft);
    if (paddingLeft >= 16) return paddingLeft;

    const outer = document.querySelector(outerSelector);
    if (!outer) throw new Error(`missing ${outerSelector}`);
    return inner.getBoundingClientRect().left - outer.getBoundingClientRect().left;
  }, outerSelector);
  assert(inset >= 16, `${label} content inset must be at least 16px; received ${inset}px`);
}

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));

  await page.goto(`${baseUrl}/admin/profile`);
  await page.waitForURL(`${baseUrl}/signin`);
  const email = page.getByLabel("Email");
  const password = page.getByLabel("Password");
  await assert.doesNotReject(() => email.waitFor());
  await assert.doesNotReject(() => password.waitFor());
  assert.equal(await page.locator(".focused-panel > .focused-shell").count(), 0);
  assert.equal(await page.locator(".focused-panel > .panel").count(), 0);
  await assertInset(page, ".focused-panel", ".focused-panel > form", "sign-in form");

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
  const legacyProfile = await page.request.get(`${baseUrl}/profile`);
  assert.equal(legacyProfile.status(), 404, "legacy /profile route must not remain exposed");
  await page.getByRole("link", { name: "Profile" }).click();
  await page.waitForURL(`${baseUrl}/admin/profile`);
  await page.locator(".workbench .rail").waitFor();
  await page.locator(".workbench .workspace").waitFor();
  await assertInset(page, ".panel", ".panel > form.panel__body", "profile form");
  await page.getByLabel("Name").fill("Scaffold Administrator Updated");
  await page.getByRole("button", { name: "Save profile" }).click();
  await page.waitForURL(`${baseUrl}/admin/profile`);
  assert.equal(await page.getByLabel("Name").inputValue(), "Scaffold Administrator Updated");
  await page.goto(`${baseUrl}/admin/users`);
  await page.getByRole("heading", { name: "Users" }).waitFor();
  await page.getByRole("link", { name: "Create user" }).click();
  await page.waitForURL(`${baseUrl}/admin/users/new`);
  await assertInset(page, ".panel", ".panel > form.panel__body", "user form");
  await page.getByLabel("Name").fill("Second Operator");
  await page.getByLabel("Email").fill("second@example.com");
  await page.getByLabel("Password").fill("second-password");
  await page.getByLabel("Roles").fill("user");
  await page.getByRole("button", { name: "Save user" }).click();
  await page.waitForURL(/\/admin\/users\/[^/]+$/);
  await page.getByText("second@example.com").waitFor();
  await assertInset(page, ".panel", ".panel > .panel__body", "user detail");
  await page.goto(`${baseUrl}/admin/users?filter=second@example.com`);
  await page.getByRole("link", { name: "Clear" }).click();
  await page.waitForURL(`${baseUrl}/admin/users`);
  await page.goto(`${baseUrl}/admin/products`);
  const filter = page.getByLabel("Filter");
  const apply = page.getByRole("button", { name: "Apply" });
  const controlHeights = await Promise.all([
    filter.evaluate((element) => element.getBoundingClientRect().height),
    apply.evaluate((element) => element.getBoundingClientRect().height),
  ]);
  assert.equal(
    controlHeights[0],
    controlHeights[1],
    `filter input (${controlHeights[0]}px) and Apply button (${controlHeights[1]}px) must match`,
  );
  await page.getByRole("link", { name: "New Product" }).click();
  await page.getByLabel("Identifier").fill("browser-created-product");
  await page.getByLabel("Name").fill("Browser Created Product");
  await page.getByRole("button", { name: "Save Product" }).click();
  await page.waitForURL(`${baseUrl}/admin/products/browser-created-product`);
  await page.getByRole("heading", { name: "Browser Created Product" }).waitFor();
  assert.equal(errors.length, 0, errors.join("\n"));
} finally {
  await browser.close();
}
