import { test, expect } from "@playwright/test";

test.describe("Navigation and routing", () => {
  test("/ renders landing page with navbar and footer", async ({ page }) => {
    await page.goto("/");

    await expect(page.locator("nav")).toBeVisible();
    await expect(page.locator("footer")).toBeVisible();
    await expect(page.locator("h1")).toContainText("Unlock your");
  });

  test("/blog renders blog page", async ({ page }) => {
    await page.goto("/blog");

    await expect(page.locator("text=Blog coming soon")).toBeVisible();
    await expect(page.locator("nav")).toBeVisible();
    await expect(page.locator("footer")).toBeVisible();
  });

  test("/blog has back link to home", async ({ page }) => {
    await page.goto("/blog");

    const backLink = page.locator("text=Back to home");
    await expect(backLink).toBeVisible();

    await backLink.click();
    await expect(page.locator("h1")).toContainText("Unlock your");
  });

  test("navbar shows on both routes", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("nav")).toContainText("LocalPush");

    await page.goto("/blog");
    await expect(page.locator("nav")).toContainText("LocalPush");
  });
});
