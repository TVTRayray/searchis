import { Snippet, SearchMatchResult } from '../types/snippet';

export function searchSnippets(snippets: Snippet[], query: string): SearchMatchResult[] {
  const trimmed = query.trim().toLowerCase();
  
  if (!trimmed) {
    // Return all snippets sorted by Pinned first, then usageCount / lastUsedAt
    return snippets.map(s => ({
      snippet: s,
      score: s.pinned ? 1000 + s.usageCount : s.usageCount,
      matchType: 'exact-key' as const
    })).sort((a, b) => b.score - a.score);
  }

  const results: SearchMatchResult[] = [];

  for (const s of snippets) {
    const keyLower = s.key.toLowerCase();
    const titleLower = s.title.toLowerCase();
    const contentLower = s.content.toLowerCase();
    const aliasesLower = s.aliases.map(a => a.toLowerCase());
    const tagsLower = s.tags.map(t => t.toLowerCase());

    // 1. Key Exact Match
    if (keyLower === trimmed) {
      results.push({
        snippet: s,
        score: 10000 + (s.pinned ? 500 : 0) + s.usageCount,
        matchType: 'exact-key',
        matchedText: s.key
      });
      continue;
    }

    // 2. Key Prefix Match
    if (keyLower.startsWith(trimmed)) {
      results.push({
        snippet: s,
        score: 8000 + (s.pinned ? 300 : 0) - keyLower.length + s.usageCount,
        matchType: 'prefix-key',
        matchedText: s.key
      });
      continue;
    }

    // 3. Key Substring Match
    if (keyLower.includes(trimmed)) {
      results.push({
        snippet: s,
        score: 6000 + (s.pinned ? 200 : 0) + s.usageCount,
        matchType: 'prefix-key',
        matchedText: s.key
      });
      continue;
    }

    // 4. Aliases Exact/Prefix Match
    const matchedAlias = aliasesLower.find(a => a === trimmed || a.startsWith(trimmed) || a.includes(trimmed));
    if (matchedAlias) {
      results.push({
        snippet: s,
        score: 5000 + (s.pinned ? 150 : 0) + s.usageCount,
        matchType: 'alias',
        matchedText: matchedAlias
      });
      continue;
    }

    // 5. Title Match
    if (titleLower.includes(trimmed)) {
      results.push({
        snippet: s,
        score: 4000 + (s.pinned ? 100 : 0) + s.usageCount,
        matchType: 'title',
        matchedText: s.title
      });
      continue;
    }

    // 6. Tag Match
    const matchedTag = tagsLower.find(t => t.includes(trimmed));
    if (matchedTag) {
      results.push({
        snippet: s,
        score: 3000 + (s.pinned ? 80 : 0) + s.usageCount,
        matchType: 'tag',
        matchedText: matchedTag
      });
      continue;
    }

    // 7. Fulltext Content Match
    if (contentLower.includes(trimmed)) {
      results.push({
        snippet: s,
        score: 2000 + (s.pinned ? 50 : 0) + s.usageCount,
        matchType: 'content',
        matchedText: s.content
      });
      continue;
    }
  }

  // Sort by score descending
  return results.sort((a, b) => b.score - a.score);
}
