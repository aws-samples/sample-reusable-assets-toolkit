// SPDX-License-Identifier: MIT

import { marked } from 'marked';
import { highlight } from '@/lib/highlight';

marked.use({
  async: false,
  breaks: false,
  gfm: true,
  renderer: {
    code({ text, lang }: { text: string; lang?: string }): string {
      const html = highlight(text, lang ?? null);
      return `<pre class="overflow-auto rounded border border-gray-200 bg-gray-50 px-3 py-2 font-mono text-xs leading-relaxed"><code>${html}</code></pre>`;
    },
  },
});

export function renderMarkdown(src: string): string {
  return marked.parse(src) as string;
}
