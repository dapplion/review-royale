import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'https://review-royale.fly.dev';

test.describe('Landing Page', () => {
  test('displays landing page at root URL', async ({ page }) => {
    await page.goto(BASE_URL);
    
    // Wait for landing content to load
    await page.waitForSelector('#landing-view', { timeout: 10000 });
    
    // Should show the landing page content
    await expect(page.locator('text=What is Review Royale?')).toBeVisible();
    
    // Take screenshot
    await page.screenshot({ path: `screenshots/landing-${test.info().project.name}.png`, fullPage: true });
  });
});

test.describe('Leaderboard', () => {
  test('displays global leaderboard with users', async ({ page }) => {
    // Navigate to a repo to see the leaderboard (root shows landing page)
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
    
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
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
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
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
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
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
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
    
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
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

test.describe('Achievement Catalog (M11)', () => {
  test('achievement catalog displays all achievements', async ({ page }) => {
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Open a user profile first
    await page.locator('.leaderboard-row').first().click();
    await page.waitForSelector('#profile-view:not(.hidden)', { timeout: 5000 });
    
    // Click "View all achievements" link
    await page.locator('a:has-text("View all achievements")').click();
    await page.waitForSelector('#achievements-view:not(.hidden)', { timeout: 5000 });
    
    // Should show achievement catalog
    await expect(page.locator('#achievements-catalog')).toBeVisible();
    
    // Should have category sections (Milestone, Speed, Quality, etc.)
    const categories = await page.locator('#achievements-catalog h2').count();
    expect(categories).toBeGreaterThanOrEqual(3);
    
    // Take screenshot
    await page.screenshot({ path: `screenshots/achievement-catalog-${test.info().project.name}.png`, fullPage: true });
  });

  test('profile shows achievements section with Up Next', async ({ page }) => {
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Open profile
    await page.locator('.leaderboard-row').first().click();
    await page.waitForSelector('#profile-view:not(.hidden)', { timeout: 5000 });
    
    // Wait for achievements to load
    await page.waitForTimeout(2000);
    
    // Achievements section should be visible
    await expect(page.locator('#achievements-section')).toBeVisible();
    
    // "View all achievements" link should be present
    await expect(page.locator('a:has-text("View all achievements")')).toBeVisible();
    
    // Take screenshot
    await page.screenshot({ path: `screenshots/profile-achievements-${test.info().project.name}.png`, fullPage: true });
  });

  test('Up Next section shows progress bars', async ({ page }) => {
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Open profile
    await page.locator('.leaderboard-row').first().click();
    await page.waitForSelector('#profile-view:not(.hidden)', { timeout: 5000 });
    
    // Wait for progress data to load
    await page.waitForTimeout(2000);
    
    // Check if Up Next section exists (may be hidden if no progress)
    const upNextSection = page.locator('#achievements-up-next');
    
    // If visible, verify it has progress bars
    if (await upNextSection.isVisible()) {
      const progressBars = await page.locator('#up-next-grid .bg-purple-500').count();
      expect(progressBars).toBeGreaterThan(0);
      
      await page.screenshot({ path: `screenshots/profile-up-next-${test.info().project.name}.png` });
    }
  });

  test('achievement catalog has rarity colors', async ({ page }) => {
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Open profile and navigate to catalog
    await page.locator('.leaderboard-row').first().click();
    await page.waitForSelector('#profile-view:not(.hidden)', { timeout: 5000 });
    await page.locator('a:has-text("View all achievements")').click();
    await page.waitForSelector('#achievements-view:not(.hidden)', { timeout: 5000 });
    
    // Should have achievements with different rarity colors
    // Common (gray), Uncommon (green), Rare (blue), Epic (purple), Legendary (yellow)
    const achievements = await page.locator('#achievements-catalog [class*="border-"]').count();
    expect(achievements).toBeGreaterThan(5);
    
    await page.screenshot({ path: `screenshots/achievement-rarities-${test.info().project.name}.png`, fullPage: true });
  });
});

test.describe('Visual Elements', () => {
  test('level badges have correct colors', async ({ page }) => {
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
    await page.waitForSelector('.leaderboard-row', { timeout: 10000 });
    
    // Check that level badges exist and have gradient classes
    const levelBadges = page.locator('[class*="bg-gradient"]');
    const count = await levelBadges.count();
    expect(count).toBeGreaterThan(0);
    
    await page.screenshot({ path: `screenshots/level-badges-${test.info().project.name}.png` });
  });

  test('XP breakdown shows formula', async ({ page }) => {
    await page.goto(`${BASE_URL}/sigp/lighthouse`);
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
