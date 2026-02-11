import { test, expect } from "@playwright/test";

test.describe("Signup modal flow", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("CTA button opens modal with intent capture", async ({ page }) => {
    await page
      .locator("button", { hasText: "Become an Early Tester" })
      .first()
      .click();

    // Modal should be visible with intent question
    await expect(
      page.locator("text=What are you most excited to try?")
    ).toBeVisible();
  });

  test("shows all 6 intent options", async ({ page }) => {
    await page
      .locator("button", { hasText: "Become an Early Tester" })
      .first()
      .click();

    const options = [
      "Track my Claude Code token spend",
      "Unlock my Apple data",
      "Replace my cron jobs",
      "Feed my self-hosted AI agents",
      "Push Mac data to a Google Sheet",
      "Something else",
    ];

    for (const option of options) {
      await expect(page.locator(`text=${option}`)).toBeVisible();
    }
  });

  test("'Something else' reveals text input", async ({ page }) => {
    await page
      .locator("button", { hasText: "Become an Early Tester" })
      .first()
      .click();

    // Click "Something else" radio
    await page.locator("text=Something else").click();

    // Text input should appear
    await expect(
      page.locator('input[placeholder="Tell us more..."]')
    ).toBeVisible();
  });

  test("continue button is disabled without selection", async ({ page }) => {
    await page
      .locator("button", { hasText: "Become an Early Tester" })
      .first()
      .click();

    const continueBtn = page.locator("button", { hasText: "Continue" });
    await expect(continueBtn).toBeDisabled();
  });

  test("selecting intent and continuing shows auth step", async ({ page }) => {
    await page
      .locator("button", { hasText: "Become an Early Tester" })
      .first()
      .click();

    // Select an intent
    await page.locator("text=Track my Claude Code token spend").click();

    // Click continue
    await page.locator("button", { hasText: "Continue" }).click();

    // Should show auth buttons
    await expect(page.locator("text=Sign in with GitHub")).toBeVisible();
    await expect(page.locator("text=Sign in with Discord")).toBeVisible();
    await expect(page.locator("text=Sign in with Google")).toBeVisible();
  });

  test("modal closes on backdrop click", async ({ page }) => {
    await page
      .locator("button", { hasText: "Become an Early Tester" })
      .first()
      .click();

    await expect(
      page.locator("text=What are you most excited to try?")
    ).toBeVisible();

    // Click the backdrop (the outer fixed div)
    await page.locator(".fixed.inset-0").click({ position: { x: 10, y: 10 } });

    await expect(
      page.locator("text=What are you most excited to try?")
    ).not.toBeVisible();
  });

  test("modal resets state on reopen", async ({ page }) => {
    // Open and select intent
    await page
      .locator("button", { hasText: "Become an Early Tester" })
      .first()
      .click();
    await page.locator("text=Track my Claude Code token spend").click();
    await page.locator("button", { hasText: "Continue" }).click();

    // Close
    await page.locator('[aria-label="Close modal"]').click();

    // Reopen — should be back to intent step
    await page
      .locator("button", { hasText: "Become an Early Tester" })
      .first()
      .click();
    await expect(
      page.locator("text=What are you most excited to try?")
    ).toBeVisible();
  });
});
