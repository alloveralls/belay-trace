const { test, expect } = require('@playwright/test');
const { execFileSync, spawn } = require('node:child_process');
const { createHash } = require('node:crypto');
const { existsSync, mkdtempSync, readFileSync, readdirSync, renameSync, writeFileSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const http = require('node:http');
const readline = require('node:readline');

const belay = resolve(__dirname, '../../target/debug/belay');
const run = (program, args, cwd) => execFileSync(program, args, { cwd, encoding: 'utf8', env: { ...process.env, GIT_TERMINAL_PROMPT: '0' } }).trim();
function createdId(output) { const match = output.match(/Created (\S+)/); if (!match) throw new Error(`missing created ID: ${output}`); return match[1]; }
function digestTree(root) {
  const hash = createHash('sha256');
  function visit(path, relative) {
    for (const item of readdirSync(path, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const child = join(path, item.name); const rel = join(relative, item.name);
      if (item.name.endsWith('-wal') || item.name.endsWith('-shm')) continue;
      if (item.isDirectory()) visit(child, rel); else { hash.update(rel); hash.update(readFileSync(child)); }
    }
  }
  visit(join(root, '.belay', 'entries'), 'entries'); visit(join(root, '.belay', 'evidence'), 'evidence');
  hash.update(readFileSync(join(root, '.belay', 'state', 'belay.sqlite'))); return hash.digest('hex');
}
async function startBrowse(cwd) {
  const child = spawn(belay, ['browse'], { cwd, stdio: ['ignore', 'pipe', 'pipe'] });
  const lines = readline.createInterface({ input: child.stdout });
  const url = await Promise.race([
    new Promise((resolveUrl, reject) => { lines.on('line', line => { const match = line.match(/Belay Browse: (http:\/\/127\.0\.0\.1:\d+\/)/); if (match) resolveUrl(match[1]); }); child.once('exit', code => reject(new Error(`browse exited before URL (${code})`))); }),
    new Promise((_, reject) => setTimeout(() => reject(new Error('browse startup timeout')), 15000))
  ]);
  return { child, url };
}
function requestStatus(url, headers) {
  return new Promise((resolveStatus, reject) => {
    const request = http.get(url, { headers }, response => { response.resume(); response.on('end', () => resolveStatus(response.statusCode)); });
    request.on('error', reject);
  });
}

test('Library, Reader, provenance, staged Explore, reload, and read-only invariant', async ({ page }) => {
  const root = mkdtempSync(join(tmpdir(), 'belay-browse-e2e-'));
  run('git', ['init', '-q'], root); run('git', ['config', 'user.email', 'fixture@example.invalid'], root); run('git', ['config', 'user.name', 'Fixture'], root);
  writeFileSync(join(root, 'code.txt'), 'first\n'); run('git', ['add', 'code.txt'], root); run('git', ['commit', '-qm', 'fixture commit'], root);
  const commit = run('git', ['rev-parse', 'HEAD'], root); run(belay, ['init'], root);
  const goal = createdId(run(belay, ['add', 'goal', '--title', 'Browse fixture goal'], root));
  const work = createdId(run(belay, ['add', 'work', '--title', 'Browse fixture work', '--body', 'Searchable provenance body.'], root));
  run(belay, ['link', work, goal, '--relation', 'fulfills'], root);
  run(belay, ['verify', 'record', '--kind', 'test', '--verdict', 'pass', '--source', 'fixture', '--summary', 'Fixture passed', '--commit', commit, '--verifies', work], root);
  const before = digestTree(root); const server = await startBrowse(root);
  const database = join(root, '.belay', 'state', 'belay.sqlite');
  const heldDatabase = `${database}.held`;
  try {
    const response = await page.goto(server.url);
    const headers = await response.allHeaders();
    expect(headers['content-security-policy']).toContain("default-src 'none'");
    expect(headers['x-content-type-options']).toBe('nosniff');
    expect(headers['x-frame-options']).toBe('DENY');
    expect(await requestStatus(server.url, { Host: 'attacker.invalid' })).toBe(403);
    expect(await requestStatus(server.url, { Origin: 'https://attacker.invalid' })).toBe(403);
    await expect(page.getByRole('heading', { name: 'Library' })).toBeVisible();
    await page.getByPlaceholder('Search entries').fill('Searchable'); await page.getByRole('button', { name: 'Search' }).click();
    await page.getByRole('link', { name: 'Browse fixture work' }).press('Enter'); await expect(page.getByText('Fixture passed')).toBeVisible();
    await page.getByRole('link', { name: /EVD-/ }).click(); await page.getByRole('link', { name: 'Inspect captured commit' }).click();
    await expect(page.getByText(/Files below were changed/)).toBeVisible(); await page.getByRole('link', { name: 'code.txt' }).click();
    await expect(page.getByText(/No direct Entry relationship/)).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Diff' })).toBeVisible();
    await page.goto(`${server.url}explore`); await expect(page.locator('#graph')).toBeVisible();
    await expect.poll(() => page.locator('#graph canvas').count()).toBeGreaterThan(0);
    const stages = await page.evaluate(async workId => {
      const load = focus => fetch(`/api/explore?focus=${encodeURIComponent(focus)}`).then(response => response.json());
      const initial = await load('all');
      const entry = await load(`entry:${workId}`);
      const evidenceId = entry.nodes.find(node => node.data.kind === 'evidence').data.id;
      const evidence = await load(evidenceId);
      const commitId = evidence.nodes.find(node => node.data.kind === 'commit').data.id;
      const commit = await load(commitId);
      return { initial, entry, evidence, commit };
    }, work);
    expect(stages.initial.nodes.every(node => node.data.kind === 'entry')).toBe(true);
    expect(stages.initial.nodes.every(node => node.data.entry_type === 'goal')).toBe(true);
    expect(stages.initial.nodes.some(node => node.data.id === `entry:${goal}`)).toBe(true);
    expect(stages.initial.edges).toHaveLength(0);
    expect(stages.entry.nodes.some(node => node.data.kind === 'evidence')).toBe(true);
    expect(stages.evidence.nodes.some(node => node.data.kind === 'commit')).toBe(true);
    expect(stages.commit.nodes.some(node => node.data.kind === 'file')).toBe(true);
    expect(await page.evaluate(() => fetch('/api/reload', { method: 'POST' }).then(response => response.status))).toBe(403);
    renameSync(database, heldDatabase);
    await page.getByRole('button', { name: 'Reload' }).click();
    await expect(page.getByText(/previous snapshot retained/)).toBeVisible();
    await expect(page.locator('#graph')).toBeVisible();
    renameSync(heldDatabase, database);
    await page.getByRole('button', { name: 'Reload' }).click(); await expect(page.getByText('Snapshot reloaded atomically.')).toBeVisible();
  } finally { if (existsSync(heldDatabase)) renameSync(heldDatabase, database); server.child.kill('SIGTERM'); }
  expect(digestTree(root)).toBe(before);
});
