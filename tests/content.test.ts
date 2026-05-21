import { describe, it, expect } from 'vitest';
import {
  filterSections,
  highlight,
  escapeHtml
} from '../src/renderer/overlay/main';
import type { ContentSection } from '../src/shared/types';

const SECTIONS: ContentSection[] = [
  {
    section: 'Véhicules',
    items: [
      { key: 'HT',  value: 'Semi-chenillé' },
      { key: 'AC',  value: 'Voiture Blindée' },
      { key: 'BT',  value: 'Char de Bataille' }
    ]
  },
  {
    section: 'Structures',
    items: [
      { key: 'FOB', value: 'Base Opérationnelle Avancée' },
      { key: 'BB',  value: 'Base Bunker' }
    ]
  }
];

describe('filterSections', () => {
  it('returns all sections when query is empty', () => {
    expect(filterSections(SECTIONS, '')).toEqual(SECTIONS);
  });

  it('filters by key (case-insensitive)', () => {
    const result = filterSections(SECTIONS, 'ht');
    expect(result).toHaveLength(1);
    expect(result[0].items).toHaveLength(1);
    expect(result[0].items[0].key).toBe('HT');
  });

  it('filters by value (case-insensitive)', () => {
    const result = filterSections(SECTIONS, 'bunker');
    expect(result).toHaveLength(1);
    expect(result[0].items[0].key).toBe('BB');
  });

  it('removes sections with no matching items', () => {
    const result = filterSections(SECTIONS, 'bunker');
    expect(result.every((s) => s.section !== 'Véhicules')).toBe(true);
  });

  it('returns empty array when nothing matches', () => {
    expect(filterSections(SECTIONS, 'zzznomatch')).toHaveLength(0);
  });

  it('does not mutate the original sections', () => {
    filterSections(SECTIONS, 'ht');
    expect(SECTIONS[0].items).toHaveLength(3);
  });

  it('returns matching items from multiple sections', () => {
    // 'b' matches 'BT' key, 'BB' key, and 'Base Bunker'/'Base Opérationnelle' values
    const result = filterSections(SECTIONS, 'base');
    expect(result).toHaveLength(1);
    expect(result[0].section).toBe('Structures');
    expect(result[0].items).toHaveLength(2);
  });

  it('handles an empty sections array', () => {
    expect(filterSections([], 'anything')).toHaveLength(0);
  });

  it('matches partial value in the middle of a string', () => {
    const result = filterSections(SECTIONS, 'opérat');
    expect(result).toHaveLength(1);
    expect(result[0].items[0].key).toBe('FOB');
  });

  it('treats a whitespace-only query as a filter (nothing matches spaces)', () => {
    expect(filterSections(SECTIONS, '   ')).toHaveLength(0);
  });
});

describe('escapeHtml', () => {
  it('escapes special HTML characters', () => {
    expect(escapeHtml('<script>alert("xss")</script>')).toBe(
      '&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;'
    );
  });

  it('escapes ampersands', () => {
    expect(escapeHtml('foo & bar')).toBe('foo &amp; bar');
  });

  it('leaves plain text unchanged', () => {
    expect(escapeHtml('hello world')).toBe('hello world');
  });

  it('returns empty string unchanged', () => {
    expect(escapeHtml('')).toBe('');
  });

  it('leaves single quotes unchanged', () => {
    expect(escapeHtml("it's fine")).toBe("it's fine");
  });

  it('escapes all HTML entities in a complex string', () => {
    expect(escapeHtml('<a href="url">text & more</a>')).toBe(
      '&lt;a href=&quot;url&quot;&gt;text &amp; more&lt;/a&gt;'
    );
  });
});

describe('highlight', () => {
  it('wraps matches in <mark>', () => {
    expect(highlight('Base Bunker', 'bunker')).toBe('Base <mark>Bunker</mark>');
  });

  it('is case-insensitive', () => {
    expect(highlight('HELLO', 'hello')).toBe('<mark>HELLO</mark>');
  });

  it('escapes HTML before highlighting', () => {
    const result = highlight('<div>', 'div');
    expect(result).toContain('&lt;');
    expect(result).not.toContain('<div>');
  });

  it('returns escaped text unchanged when query is empty', () => {
    expect(highlight('hello', '')).toBe('hello');
  });

  it('wraps all occurrences when the query appears multiple times', () => {
    expect(highlight('foo foo', 'foo')).toBe('<mark>foo</mark> <mark>foo</mark>');
  });

  it('returns escaped text unchanged when query has no match', () => {
    expect(highlight('hello', 'xyz')).toBe('hello');
  });

  it('highlights inside HTML-escaped content correctly', () => {
    // '<div>' → '&lt;div&gt;'; query 'div' matches the literal 'div' substring
    expect(highlight('<div>', 'div')).toBe('&lt;<mark>div</mark>&gt;');
  });

  it('highlights a full-string match', () => {
    expect(highlight('hello', 'hello')).toBe('<mark>hello</mark>');
  });
});

describe('highlight — regex safety', () => {
  it('handles regex special chars in query without throwing', () => {
    expect(() => highlight('foo (bar)', '(bar)')).not.toThrow();
  });

  it('handles a dot in query without matching every character', () => {
    expect(highlight('abcXYZ', 'a.c')).toBe('abcXYZ');
  });
});
