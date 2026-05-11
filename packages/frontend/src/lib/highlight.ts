// SPDX-License-Identifier: MIT

import Prism from 'prismjs';
// Load order matters: extenders after base languages.
// jsx extends markup + javascript, tsx extends jsx + typescript.
import 'prismjs/components/prism-typescript';
import 'prismjs/components/prism-jsx';
import 'prismjs/components/prism-tsx';
import 'prismjs/components/prism-rust';
import 'prismjs/components/prism-python';
import 'prismjs/components/prism-go';
import 'prismjs/components/prism-java';
import 'prismjs/components/prism-bash';
import 'prismjs/components/prism-json';
import 'prismjs/components/prism-yaml';
import 'prismjs/components/prism-toml';
import 'prismjs/components/prism-markdown';
import 'prismjs/components/prism-sql';

// Normalise backend language identifiers to Prism language keys.
const LANG_MAP: Record<string, string> = {
  rust: 'rust',
  rs: 'rust',
  typescript: 'typescript',
  ts: 'typescript',
  tsx: 'tsx',
  javascript: 'javascript',
  js: 'javascript',
  jsx: 'jsx',
  python: 'python',
  py: 'python',
  go: 'go',
  golang: 'go',
  java: 'java',
  bash: 'bash',
  sh: 'bash',
  shell: 'bash',
  zsh: 'bash',
  json: 'json',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
  markdown: 'markdown',
  md: 'markdown',
  sql: 'sql',
};

const escapeHtml = (s: string): string =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

export function highlight(code: string, language: string | null): string {
  const key = language?.toLowerCase();
  const prismLang = key ? LANG_MAP[key] : undefined;
  const grammar = prismLang ? Prism.languages[prismLang] : undefined;
  if (!grammar || !prismLang) {
    return escapeHtml(code);
  }
  return Prism.highlight(code, grammar, prismLang);
}
