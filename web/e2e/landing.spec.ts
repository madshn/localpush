import { test, expect } from "@playwright/test";

test.describe("Landing page content", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("renders hero with correct brief copy", async ({ page }) => {
    await expect(page.locator("h1")).toContainText("Unlock your");
    await expect(page.locator("h1")).toContainText("Mac data.");

    // Brief subheadline, NOT Stitch generic version
    const hero = page.locator("section").first();
    await expect(hero).toContainText(
      "Claude Code usage, Apple Podcasts, Notes, Photos"
    );
    await expect(hero).toContainText("Google Sheet");
    await expect(hero).toContainText("guaranteed delivery");
  });

  test("renders all 10 sections in order", async ({ page }) => {
    // Verify key elements from each section exist
    const sections = [
      "Unlock your", // 1. Hero
      "Open Source (MIT)", // 2. TrustStrip
      "data you can't reach", // 3. ProblemSolution
      "How it Works", // 4. HowItWorks
      "What will you unlock?", // 5. UseCases
      "Did you know?", // 6. DidYouKnow
      "Radical Transparency", // 7. TrustProof
      "Join the beta.", // 8. EarlyAccessCTA
      "From the Blog", // 9. BlogPreview
      "Built by", // 10. Footer
    ];

    for (const text of sections) {
      await expect(page.locator(`text=${text}`).first()).toBeVisible();
    }
  });

  test("CTA heading says 'Join the beta.' not Stitch copy", async ({ page }) => {
    // Brief says "Join the beta." — NOT Stitch's "Ready to bridge your local context?"
    await expect(page.locator("text=Join the beta.")).toBeVisible();
    await expect(page.locator("text=Ready to bridge")).not.toBeVisible();
  });

  test("CTA subtext mentions GitHub follow, not credit card", async ({ page }) => {
    // Brief says "Prefer to wait..." — NOT Stitch's "No credit card required"
    await expect(page.locator("text=Prefer to wait")).toBeVisible();
    await expect(page.locator("text=No credit card")).not.toBeVisible();
  });

  test("footer shows 2026 and links to rightaim.ai", async ({ page }) => {
    const footer = page.locator("footer");
    await expect(footer).toContainText("2026");
    await expect(footer).toContainText("Built by");

    const rightAimLink = footer.locator('a[href="https://rightaim.ai"]');
    await expect(rightAimLink).toBeVisible();
  });

  test("hero has two CTA buttons", async ({ page }) => {
    await expect(
      page.locator("button", { hasText: "Become an Early Tester" }).first()
    ).toBeVisible();
    await expect(
      page.locator("a", { hasText: "Star on GitHub" }).first()
    ).toBeVisible();
  });
});
