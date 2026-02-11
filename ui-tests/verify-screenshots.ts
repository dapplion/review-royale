#!/usr/bin/env npx ts-node
/**
 * AI Vision Verification for UI Screenshots
 * 
 * Analyzes Playwright screenshots using Claude's vision model to verify:
 * - Layout is correct and aligned
 * - Text is readable and not truncated
 * - Colors/contrast meet accessibility standards
 * - No visual glitches or broken elements
 * 
 * Usage: npx ts-node verify-screenshots.ts [--screenshot <path>]
 */

import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';
import Anthropic from '@anthropic-ai/sdk';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SCREENSHOTS_DIR = path.join(__dirname, 'screenshots');

interface VerificationResult {
  screenshot: string;
  passed: boolean;
  issues: string[];
  suggestions: string[];
}

const VERIFICATION_PROMPT = `You are a UI/UX quality assurance expert reviewing a screenshot of a web application leaderboard/profile page.

Analyze this screenshot and check for:
1. **Layout Issues**: Elements misaligned, overlapping, or cut off
2. **Text Readability**: Text too small, truncated, or illegible
3. **Visual Hierarchy**: Clear distinction between headings, data, and actions
4. **Contrast/Colors**: Adequate contrast for readability (dark theme)
5. **Data Display**: Numbers, usernames, and stats displayed clearly
6. **Responsive Issues**: Content properly sized for the viewport
7. **Broken Elements**: Missing images, icons, or placeholder text showing

Respond in this JSON format:
{
  "passed": true/false,
  "issues": ["list of actual problems found"],
  "suggestions": ["optional improvements, not blockers"]
}

Be strict about real issues but don't flag stylistic preferences. An empty issues array means it passed.`;

async function verifyScreenshot(client: Anthropic, screenshotPath: string): Promise<VerificationResult> {
  const imageData = fs.readFileSync(screenshotPath);
  const base64Image = imageData.toString('base64');
  const mediaType = 'image/png';

  const response = await client.messages.create({
    model: 'claude-sonnet-4-20250514',
    max_tokens: 1024,
    messages: [
      {
        role: 'user',
        content: [
          {
            type: 'image',
            source: {
              type: 'base64',
              media_type: mediaType,
              data: base64Image,
            },
          },
          {
            type: 'text',
            text: VERIFICATION_PROMPT,
          },
        ],
      },
    ],
  });

  // Extract text content from response
  const textContent = response.content.find(c => c.type === 'text');
  if (!textContent || textContent.type !== 'text') {
    throw new Error('No text response from Claude');
  }

  // Parse JSON from response
  const jsonMatch = textContent.text.match(/\{[\s\S]*\}/);
  if (!jsonMatch) {
    console.warn(`Could not parse JSON from response for ${screenshotPath}`);
    return {
      screenshot: path.basename(screenshotPath),
      passed: false,
      issues: ['Failed to parse AI response'],
      suggestions: [],
    };
  }

  const result = JSON.parse(jsonMatch[0]);
  return {
    screenshot: path.basename(screenshotPath),
    passed: result.passed,
    issues: result.issues || [],
    suggestions: result.suggestions || [],
  };
}

async function main() {
  // Check for API key
  if (!process.env.ANTHROPIC_API_KEY) {
    console.error('❌ ANTHROPIC_API_KEY environment variable required');
    process.exit(1);
  }

  const client = new Anthropic();

  // Get screenshots to verify
  let screenshots: string[] = [];
  
  const singleScreenshot = process.argv.find((arg, i) => 
    process.argv[i - 1] === '--screenshot'
  );

  if (singleScreenshot) {
    screenshots = [singleScreenshot];
  } else {
    // Get all PNG files in screenshots directory
    if (!fs.existsSync(SCREENSHOTS_DIR)) {
      console.error(`❌ Screenshots directory not found: ${SCREENSHOTS_DIR}`);
      console.log('Run tests first: npm test');
      process.exit(1);
    }
    screenshots = fs.readdirSync(SCREENSHOTS_DIR)
      .filter(f => f.endsWith('.png'))
      .map(f => path.join(SCREENSHOTS_DIR, f));
  }

  if (screenshots.length === 0) {
    console.log('No screenshots to verify');
    process.exit(0);
  }

  console.log(`\n🔍 Verifying ${screenshots.length} screenshot(s)...\n`);

  const results: VerificationResult[] = [];
  let failedCount = 0;

  for (const screenshot of screenshots) {
    const name = path.basename(screenshot);
    process.stdout.write(`  ${name} ... `);
    
    try {
      const result = await verifyScreenshot(client, screenshot);
      results.push(result);
      
      if (result.passed) {
        console.log('✅ PASS');
      } else {
        console.log('❌ FAIL');
        failedCount++;
      }
    } catch (error) {
      console.log('⚠️ ERROR');
      results.push({
        screenshot: name,
        passed: false,
        issues: [`Verification error: ${error}`],
        suggestions: [],
      });
      failedCount++;
    }
  }

  // Print detailed results
  console.log('\n' + '='.repeat(60));
  console.log('VERIFICATION RESULTS');
  console.log('='.repeat(60));

  for (const result of results) {
    console.log(`\n📸 ${result.screenshot}`);
    console.log(`   Status: ${result.passed ? '✅ PASSED' : '❌ FAILED'}`);
    
    if (result.issues.length > 0) {
      console.log('   Issues:');
      result.issues.forEach(issue => console.log(`     • ${issue}`));
    }
    
    if (result.suggestions.length > 0) {
      console.log('   Suggestions:');
      result.suggestions.forEach(sug => console.log(`     💡 ${sug}`));
    }
  }

  console.log('\n' + '='.repeat(60));
  console.log(`SUMMARY: ${results.length - failedCount}/${results.length} passed`);
  console.log('='.repeat(60) + '\n');

  // Write results to JSON for CI integration
  const resultsPath = path.join(SCREENSHOTS_DIR, 'verification-results.json');
  fs.writeFileSync(resultsPath, JSON.stringify(results, null, 2));
  console.log(`Results written to: ${resultsPath}`);

  process.exit(failedCount > 0 ? 1 : 0);
}

main().catch(console.error);
