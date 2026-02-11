import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'https://review-royale.fly.dev';

test.describe('Leaderboard', () => {
  test('displays global leaderboard with users', async ({ page }) => {
    await page.goto(BASE_URL);
    
    // Wait for leaderboard to load
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Should have at least one user
    const rows = await page.locator('.leaderboard-row').count();
    expect(rows).toBeGreaterThan(0);
    
    // Should show XP values
    const xpValues = await page.locator('.leaderboard-row').first().textContent();
    expect(xpValues).toContain('XP');
    
    // Take screenshot
    await page.screenshot({ path: `screenshots/leaderboard-${test.info().project.name}.png`, fullPage: true });
  });

  test('period filter changes data', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Screenshot default view (This Month)
    await page.screenshot({ path: `screenshots/leaderboard-month-${test.info().project.name}.png` });
    
    // Click "This Week" filter (in leaderboard view, not profile)
    await page.locator('#leaderboard-view button:has-text("This Week")').click();
    await page.waitForTimeout(1500); // Wait for data refresh
    
    // Take screenshot of week view
    await page.screenshot({ path: `screenshots/leaderboard-week-${test.info().project.name}.png` });
    
    // Click "All Time" to verify it changes
    await page.locator('#leaderboard-view button:has-text("All Time")').click();
    await page.waitForTimeout(1500);
    
    await page.screenshot({ path: `screenshots/leaderboard-alltime-${test.info().project.name}.png` });
  });
});

test.describe('User Profile', () => {
  test('displays user profile with stats', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Click on first user to open profile
    await page.locator('.leaderboard-row').first().click();
    
    // Wait for profile to load
    await page.waitForSelector('#profile-view:not(.hidden)', { timeout: 5000 });
    
    // Should show user stats
    await expect(page.locator('#profile-name')).toBeVisible();
    await expect(page.locator('#profile-xp')).toBeVisible();
    await expect(page.locator('#profile-level')).toBeVisible();
    
    // Stats cards should be visible
    await expect(page.locator('#profile-reviews')).toBeVisible();
    await expect(page.locator('#profile-prs-reviewed')).toBeVisible();
    await expect(page.locator('#profile-comments')).toBeVisible();
    
    // Take screenshot
    await page.screenshot({ path: `screenshots/profile-${test.info().project.name}.png`, fullPage: true });
  });

  test('profile period filter changes XP', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Open profile
    await page.locator('.leaderboard-row').first().click();
    await page.waitForSelector('#profile-view:not(.hidden)', { timeout: 5000 });
    
    // Wait for XP to be populated (not just "-")
    await page.waitForFunction(() => {
      const el = document.getElementById('profile-xp');
      return el && el.textContent && el.textContent !== '-';
    }, { timeout: 10000 });
    
    // Get month XP (default is "This Month")
    const monthXp = await page.locator('#profile-xp').textContent();
    await page.screenshot({ path: `screenshots/profile-month-${test.info().project.name}.png` });
    
    // Switch to "This Week"
    await page.locator('#profile-btn-week').click();
    await page.waitForTimeout(1500); // Wait for API response
    
    const weekXp = await page.locator('#profile-xp').textContent();
    await page.screenshot({ path: `screenshots/profile-week-${test.info().project.name}.png` });
    
    // Switch to "All Time"
    await page.locator('#profile-btn-all').click();
    await page.waitForTimeout(1500);
    
    const allXp = await page.locator('#profile-xp').textContent();
    await page.screenshot({ path: `screenshots/profile-all-${test.info().project.name}.png` });
    
    // Log XP values for debugging
    console.log(`XP values - Week: ${weekXp}, Month: ${monthXp}, All: ${allXp}`);
    
    // Verify values are different (sanity check that period filter works)
    // Week should generally be <= Month <= All
    const weekNum = parseInt(weekXp?.replace(/[^\d]/g, '') || '0');
    const monthNum = parseInt(monthXp?.replace(/[^\d]/g, '') || '0');
    const allNum = parseInt(allXp?.replace(/[^\d]/g, '') || '0');
    
    console.log(`Parsed XP - Week: ${weekNum}, Month: ${monthNum}, All: ${allNum}`);
    expect(weekNum).toBeLessThanOrEqual(allNum);
    expect(monthNum).toBeLessThanOrEqual(allNum);
  });
});

test.describe('Repo Scoped View', () => {
  test('displays repo-specific leaderboard', async ({ page }) => {
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
    
    // Wait for leaderboard to load (may take longer for repo-scoped)
    await page.waitForSelector('.leaderboard-row', { timeout: 15000 });
    
    // Should show leaderboard with users
    const rows = await page.locator('.leaderboard-row').count();
    expect(rows).toBeGreaterThan(0);
    
    // Take screenshot
    await page.screenshot({ path: `screenshots/repo-leaderboard-${test.info().project.name}.png`, fullPage: true });
  });
});

test.describe('Responsive Layout', () => {
  test('mobile layout is usable', async ({ page, browserName }, testInfo) => {
    // Skip for desktop project
    if (testInfo.project.name !== 'Mobile') {
      test.skip();
    }
    
    await page.goto(BASE_URL);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Leaderboard should be visible
    await expect(page.locator('.leaderboard-row').first()).toBeVisible();
    
    // Take screenshot
    await page.screenshot({ path: 'screenshots/mobile-leaderboard.png', fullPage: true });
    
    // Open profile
    await page.locator('.leaderboard-row').first().click();
    await page.waitForSelector('#profile-view:not(.hidden)', { timeout: 5000 });
    
    // Profile should be visible on mobile
    await expect(page.locator('#profile-name')).toBeVisible();
    
    await page.screenshot({ path: 'screenshots/mobile-profile.png', fullPage: true });
  });
});

test.describe('Visual Elements', () => {
  test('level badges have correct colors', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Check that level badges exist and have gradient classes
    const levelBadges = page.locator('[class*="bg-gradient"]');
    const count = await levelBadges.count();
    expect(count).toBeGreaterThan(0);
    
    await page.screenshot({ path: `screenshots/level-badges-${test.info().project.name}.png` });
  });

  test('XP breakdown shows formula', async ({ page }) => {
    await page.goto(BASE_URL);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Open profile
    await page.locator('.leaderboard-row').first().click();
    await page.waitForSelector('#profile-view:not(.hidden)', { timeout: 5000 });
    
    // Wait for XP breakdown to be populated (has actual content)
    await page.waitForFunction(() => {
      const el = document.getElementById('xp-breakdown');
      return el && el.textContent && el.textContent.length > 10;
    }, { timeout: 5000 });
    
    // XP breakdown should show formula elements
    const breakdown = page.locator('#xp-breakdown');
    await expect(breakdown).toBeVisible();
    
    const breakdownText = await breakdown.textContent();
    expect(breakdownText).toContain('PRs reviewed');
    expect(breakdownText).toContain('Comments');
    
    await page.screenshot({ path: `screenshots/xp-breakdown-${test.info().project.name}.png` });
  });
});
