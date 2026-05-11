// SPDX-License-Identifier: MIT

/**
 * Backend snippet descriptions follow a fixed 3-line format:
 *   SUMMARY: ...
 *   IDENTIFIERS: ...
 *   KEYWORDS: ...
 * Parse into structured fields. IDENTIFIERS/KEYWORDS primarily exist to
 * boost FTS retrieval, but KEYWORDS are useful as display tags.
 */
export type ParsedDescription = {
  summary: string;
  identifiers: string[];
  keywords: string[];
};

const splitList = (s: string): string[] =>
  s.split(',').map((x) => x.trim()).filter(Boolean);

export const parseDescription = (description: string): ParsedDescription => {
  let summary = '';
  let identifiers: string[] = [];
  let keywords: string[] = [];
  for (const line of description.split('\n')) {
    const summaryMatch = line.match(/^\s*SUMMARY:\s*(.*)$/);
    const idsMatch = line.match(/^\s*IDENTIFIERS:\s*(.*)$/);
    const kwsMatch = line.match(/^\s*KEYWORDS:\s*(.*)$/);
    if (summaryMatch) summary = summaryMatch[1].trim();
    else if (idsMatch) identifiers = splitList(idsMatch[1]);
    else if (kwsMatch) keywords = splitList(kwsMatch[1]);
  }
  if (!summary && !identifiers.length && !keywords.length) {
    summary = description.trim();
  }
  return { summary, identifiers, keywords };
};
