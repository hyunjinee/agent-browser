import { describe, it, expect } from 'vitest';
import { sanitizeSurrogates } from './snapshot.js';

describe('sanitizeSurrogates', () => {
  it('replaces lone high surrogate with replacement character', () => {
    const input = 'text \uD800 more';
    const result = sanitizeSurrogates(input);
    expect(result).toBe('text \uFFFD more');
  });

  it('replaces lone low surrogate with replacement character', () => {
    const input = 'text \uDC00 more';
    const result = sanitizeSurrogates(input);
    expect(result).toBe('text \uFFFD more');
  });

  it('preserves valid surrogate pairs', () => {
    const input = 'emoji: \uD83D\uDE00'; // 😀
    const result = sanitizeSurrogates(input);
    expect(result).toBe('emoji: \uD83D\uDE00');
  });

  it('does not modify strings without surrogates', () => {
    const input = 'hello world 한글 テスト';
    const result = sanitizeSurrogates(input);
    expect(result).toBe(input);
  });

  it('replaces consecutive lone surrogates', () => {
    const input = '\uD800\uD801\uD802';
    const result = sanitizeSurrogates(input);
    expect(result).toBe('\uFFFD\uFFFD\uFFFD');
  });
});
