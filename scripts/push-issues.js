import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Read wave-issues.json
const issuesFilePath = path.resolve(__dirname, '../.github/wave-issues.json');
const rawData = fs.readFileSync(issuesFilePath, 'utf8');
const issueData = JSON.parse(rawData);

const OWNER = 'TRELLIS-STELLAR';
const REPO = 'Trellis-contracts';

const LABEL_METADATA = {
  'legal': { color: '5319e7', description: 'Licensing, compliance, and legal considerations' },
  'ci': { color: 'fbca04', description: 'Continuous integration and automation workflows' },
  'community': { color: '006b75', description: 'Contributor onboarding, guidelines, and community health' },
  'security': { color: 'd93f0b', description: 'Security policies, vulnerabilities, and audits' },
  'core': { color: '1d76db', description: 'Core smart contract implementation and business logic' },
  'test': { color: 'bfdadc', description: 'Test coverage, harnesses, and test suites' },
  'refactor': { color: 'c5def5', description: 'Code cleanup, module reconciliation, and structural improvements' },
  'performance': { color: 'f9d0c4', description: 'Gas optimization, WASM size, and performance' },
  'needs discussion': { color: 'b60205', description: 'Requires architectural decision or maintainer consensus' }
};

const token = process.env.GITHUB_PAT || process.argv[2];
const isDryRun = process.argv.includes('--dry-run');

if (!token && !isDryRun) {
  console.error('Usage: node scripts/push-issues.js <GITHUB_PAT> [--dry-run]');
  console.error('Or set GITHUB_PAT environment variable.');
  process.exit(1);
}

const headers = {
  'Accept': 'application/vnd.github+json',
  'User-Agent': 'Trellis-Issue-Automation',
  'X-GitHub-Api-Version': '2022-11-28'
};

if (token) {
  headers['Authorization'] = `Bearer ${token.trim()}`;
}

async function githubRequest(url, options = {}) {
  const fullUrl = url.startsWith('http') ? url : `https://api.github.com${url}`;
  const response = await fetch(fullUrl, {
    ...options,
    headers: {
      ...headers,
      ...(options.headers || {})
    }
  });

  const text = await response.text();
  let json;
  try {
    json = JSON.parse(text);
  } catch {
    json = text;
  }

  if (!response.ok) {
    const errorMsg = json && json.message ? json.message : text;
    throw new Error(`GitHub API Error (${response.status}): ${errorMsg}`);
  }

  return json;
}

async function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  console.log(`Repository: ${OWNER}/${REPO}`);
  console.log(`Total issues to process: ${issueData.issues.length}`);
  if (isDryRun) {
    console.log('--- DRY RUN MODE (no changes will be made) ---\n');
  }

  // 1. Check existing issues
  let existingIssues = [];
  if (!isDryRun) {
    try {
      console.log('Fetching existing issues...');
      existingIssues = await githubRequest(`/repos/${OWNER}/${REPO}/issues?state=all&per_page=100`);
      console.log(`Found ${existingIssues.length} existing issues.`);
    } catch (err) {
      console.error('Warning: could not fetch existing issues:', err.message);
    }
  }

  // 2. Ensure labels exist
  if (!isDryRun) {
    console.log('Checking / creating repository labels...');
    let existingLabels = [];
    try {
      existingLabels = await githubRequest(`/repos/${OWNER}/${REPO}/labels?per_page=100`);
    } catch (err) {
      console.error('Warning: could not fetch existing labels:', err.message);
    }

    const existingLabelNames = new Set(existingLabels.map((l) => l.name.toLowerCase()));

    // Collect all labels from the issues
    const allLabels = new Set();
    for (const issue of issueData.issues) {
      for (const label of issue.labels) {
        allLabels.add(label);
      }
    }

    for (const labelName of allLabels) {
      if (!existingLabelNames.has(labelName.toLowerCase())) {
        const meta = LABEL_METADATA[labelName] || { color: 'ededed', description: '' };
        console.log(`Creating label "${labelName}"...`);
        try {
          await githubRequest(`/repos/${OWNER}/${REPO}/labels`, {
            method: 'POST',
            body: JSON.stringify({
              name: labelName,
              color: meta.color,
              description: meta.description
            })
          });
          console.log(`  ✓ Label "${labelName}" created.`);
          await sleep(500);
        } catch (err) {
          console.error(`  ✗ Failed to create label "${labelName}":`, err.message);
        }
      }
    }
  }

  // 3. Create issues
  console.log('\nCreating issues...');
  const existingTitles = new Set(existingIssues.map((i) => i.title.toLowerCase()));

  let createdCount = 0;
  let skippedCount = 0;

  for (const issue of issueData.issues) {
    const { number, stage, title, labels, body } = issue;
    console.log(`\n[#${number}] Stage ${stage}: "${title}"`);
    console.log(`  Labels: [${labels.join(', ')}]`);

    if (existingTitles.has(title.toLowerCase())) {
      console.log(`  -> Skipped: issue with this title already exists.`);
      skippedCount++;
      continue;
    }

    if (isDryRun) {
      console.log(`  -> Would create issue with ${body.length} characters in body.`);
      createdCount++;
      continue;
    }

    try {
      const created = await githubRequest(`/repos/${OWNER}/${REPO}/issues`, {
        method: 'POST',
        body: JSON.stringify({
          title,
          body,
          labels
        })
      });

      console.log(`  ✓ Created: #${created.number} -> ${created.html_url}`);
      createdCount++;
      // Wait 1.5 seconds between issues to avoid secondary rate limits
      await sleep(1500);
    } catch (err) {
      console.error(`  ✗ Error creating issue: ${err.message}`);
    }
  }

  console.log('\n========================================');
  console.log(`Summary: ${createdCount} created, ${skippedCount} skipped.`);
  console.log('========================================');
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
