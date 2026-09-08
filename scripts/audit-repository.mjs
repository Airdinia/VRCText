// Heuristic hygiene check, not a guarantee that every secret can be detected.
// Findings contain file/object identifiers, never the matched secret value.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';

const git = (...args) => execFileSync('git', args, { maxBuffer: 128 * 1024 * 1024 });
const findings = [];
const rules = [
  ['private-key', /-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/],
  ['github-token', /\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b/],
  ['cloud-access-key', /\b(?:AKIA|ASIA)[A-Z0-9]{16}\b/],
  ['service-token', /\b(?:sk-(?:proj-|ant-)?[A-Za-z0-9_-]{30,}|xox[baprs]-[A-Za-z0-9-]{20,})\b/],
  ['credential-in-url', /https?:\/\/[^\s/:]+:[^\s/@]+@/],
];
function scan(id, bytes) {
  if (bytes.includes(0)) return;
  const text = bytes.toString('utf8');
  for (const [rule, pattern] of rules) if (pattern.test(text)) findings.push({ id, rule });
}
const paths = git('ls-files', '-z', '--cached', '--others', '--exclude-standard').toString().split('\0').filter(Boolean);
for (const path of new Set(paths)) {
  if (!fs.existsSync(path) || !fs.statSync(path).isFile()) continue;
  if (/(^|\/)(?:\.env(?:\..*)?|node_modules|target|dist|\.claude|\.local-review)(?:\/|$)|\.(?:pfx|p12|pem|key|bundle)$/i.test(path)) {
    findings.push({id:path, rule:'private-or-generated-path'});
  }
  scan(path, fs.readFileSync(path));
}
let blobs = 0;
if (process.argv.includes('--history')) {
  const objects = git('rev-list', '--objects', '--branches', '--tags', '--remotes').toString().trim().split('\n').filter(Boolean);
  const ids = objects.map(line => line.split(' ')[0]);
  const metadata = execFileSync('git', ['cat-file', '--batch-check'], {input:ids.join('\n')+'\n', maxBuffer:128*1024*1024}).toString().trim().split('\n');
  for (let i = 0; i < metadata.length; i++) {
    const [id, type] = metadata[i].split(' ');
    if (type !== 'blob') continue;
    scan(objects[i], git('cat-file', 'blob', id));
    blobs++;
  }
  const commits = git('log', '--branches', '--tags', '--remotes', '--format=%H%x00%B%x00').toString();
  if (/^Co-authored-by:.*(?:anthropic|openai|claude|codex|gemini|copilot)/im.test(commits)) findings.push({id:'commit messages',rule:'ai-coauthor'});
}
console.log(JSON.stringify({ files: new Set(paths).size, historicalBlobs: blobs, findings }, null, 2));
process.exitCode = findings.length ? 1 : 0;
