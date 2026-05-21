import { describe, it, expect, vi, afterEach } from 'vitest';
import { applyOpacity, applyFontSize } from '../src/renderer/overlay/main';

// applyOpacity calls document.documentElement.style.setProperty.
// In the node test environment there is no real DOM, so we stub the global
// document before each assertion and restore it afterwards.

afterEach(() => {
  vi.unstubAllGlobals();
});

function makeDocStub() {
  const setProperty = vi.fn();
  vi.stubGlobal('document', {
    documentElement: { style: { setProperty } }
  });
  return setProperty;
}

describe('applyOpacity', () => {
  it('sets the --opacity CSS variable with the given value', () => {
    const spy = makeDocStub();
    applyOpacity(0.75);
    expect(spy).toHaveBeenCalledWith('--opacity', '0.75');
  });

  it('accepts 0 (fully transparent)', () => {
    const spy = makeDocStub();
    applyOpacity(0);
    expect(spy).toHaveBeenCalledWith('--opacity', '0');
  });

  it('accepts 1 (fully opaque)', () => {
    const spy = makeDocStub();
    applyOpacity(1);
    expect(spy).toHaveBeenCalledWith('--opacity', '1');
  });

  it('coerces the number to a string (no trailing zeros)', () => {
    const spy = makeDocStub();
    applyOpacity(0.5);
    expect(spy).toHaveBeenCalledWith('--opacity', '0.5');
  });
});

describe('applyFontSize', () => {
  it('sets the --font-size CSS variable with px unit', () => {
    const spy = makeDocStub();
    applyFontSize(14);
    expect(spy).toHaveBeenCalledWith('--font-size', '14px');
  });

  it('works with the default value (12)', () => {
    const spy = makeDocStub();
    applyFontSize(12);
    expect(spy).toHaveBeenCalledWith('--font-size', '12px');
  });

  it('works with the maximum value (18)', () => {
    const spy = makeDocStub();
    applyFontSize(18);
    expect(spy).toHaveBeenCalledWith('--font-size', '18px');
  });

  it('works with the minimum value (10)', () => {
    const spy = makeDocStub();
    applyFontSize(10);
    expect(spy).toHaveBeenCalledWith('--font-size', '10px');
  });
});
