import { describe, it, expect } from 'vitest';
import { DEFAULT_SETTINGS, mergeSettings } from '../src/shared/settings';
import type { AppSettings } from '../src/shared/types';

describe('mergeSettings', () => {
  it('returns defaults when patch is empty', () => {
    const result = mergeSettings({ ...DEFAULT_SETTINGS }, {});
    expect(result).toEqual(DEFAULT_SETTINGS);
  });

  it('overrides only patched keys', () => {
    const base: AppSettings = { ...DEFAULT_SETTINGS, opacity: 0.5 };
    const result = mergeSettings(base, { opacity: 0.7 });
    expect(result.opacity).toBe(0.7);
    expect(result.hotkey).toBe(DEFAULT_SETTINGS.hotkey);
  });

  it('does not mutate the original object', () => {
    const base: AppSettings = { ...DEFAULT_SETTINGS };
    mergeSettings(base, { opacity: 0.1 });
    expect(base.opacity).toBe(DEFAULT_SETTINGS.opacity);
  });

  it('patches multiple keys in one call', () => {
    const result = mergeSettings({ ...DEFAULT_SETTINGS }, { opacity: 0.5, width: 400, height: 600 });
    expect(result.opacity).toBe(0.5);
    expect(result.width).toBe(400);
    expect(result.height).toBe(600);
    expect(result.hotkey).toBe(DEFAULT_SETTINGS.hotkey);
  });

  it('patches all keys at once', () => {
    const full: AppSettings = {
      hotkey: 'F2', opacity: 0.5,
      width: 400, height: 600, x: 50, y: 50, fontSize: 14, activeSection: 'Combat',
      activeMode: 'lexique',
      artyInstanceId: null,
      artyLayerId: null,
      artyGroupId: null,
      artyBatteryId: null,
      artyShowAllBatteries: false,
    };
    const result = mergeSettings({ ...DEFAULT_SETTINGS }, full);
    expect(result).toEqual(full);
  });

  it('accepts opacity boundary value 0', () => {
    expect(mergeSettings({ ...DEFAULT_SETTINGS }, { opacity: 0 }).opacity).toBe(0);
  });

  it('accepts opacity boundary value 1', () => {
    expect(mergeSettings({ ...DEFAULT_SETTINGS }, { opacity: 1 }).opacity).toBe(1);
  });
});

describe('DEFAULT_SETTINGS shape', () => {
  it('has all required keys', () => {
    const keys: (keyof AppSettings)[] = ['hotkey', 'opacity', 'width', 'height', 'x', 'y', 'fontSize', 'activeSection'];
    for (const k of keys) {
      expect(DEFAULT_SETTINGS).toHaveProperty(k);
    }
  });

  it('opacity is between 0 and 1', () => {
    expect(DEFAULT_SETTINGS.opacity).toBeGreaterThan(0);
    expect(DEFAULT_SETTINGS.opacity).toBeLessThanOrEqual(1);
  });

  it('has a non-empty hotkey string', () => {
    expect(DEFAULT_SETTINGS.hotkey).toMatch(/\S/);
  });

  it('has positive dimension values', () => {
    expect(DEFAULT_SETTINGS.width).toBeGreaterThan(0);
    expect(DEFAULT_SETTINGS.height).toBeGreaterThan(0);
  });

  it('has non-negative position values', () => {
    expect(DEFAULT_SETTINGS.x).toBeGreaterThanOrEqual(0);
    expect(DEFAULT_SETTINGS.y).toBeGreaterThanOrEqual(0);
  });

  it('has a positive fontSize value', () => {
    expect(DEFAULT_SETTINGS.fontSize).toBeGreaterThan(0);
  });
});

describe('mergeSettings — new fields', () => {
  it('patches fontSize independently', () => {
    const result = mergeSettings({ ...DEFAULT_SETTINGS }, { fontSize: 16 });
    expect(result.fontSize).toBe(16);
    expect(result.opacity).toBe(DEFAULT_SETTINGS.opacity);
  });

  it('preserves fontSize when not in patch', () => {
    const result = mergeSettings({ ...DEFAULT_SETTINGS }, { opacity: 0.5 });
    expect(result.fontSize).toBe(DEFAULT_SETTINGS.fontSize);
  });

  it('defaults activeSection to null', () => {
    expect(DEFAULT_SETTINGS.activeSection).toBeNull();
  });

  it('patches activeSection to a string', () => {
    const result = mergeSettings({ ...DEFAULT_SETTINGS }, { activeSection: 'Combat' });
    expect(result.activeSection).toBe('Combat');
  });

  it('patches activeSection back to null', () => {
    const base: AppSettings = { ...DEFAULT_SETTINGS, activeSection: 'Combat' };
    const result = mergeSettings(base, { activeSection: null });
    expect(result.activeSection).toBeNull();
  });
});
