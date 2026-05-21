import { describe, it, expect } from 'vitest';
import { eventToAccelerator } from '../src/renderer/settings/main';

function fakeEvent(overrides: Partial<KeyboardEvent>): KeyboardEvent {
  return {
    key: 'F1',
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    preventDefault: () => {},
    ...overrides
  } as unknown as KeyboardEvent;
}

describe('eventToAccelerator', () => {
  it('returns plain key for function keys (uppercase)', () => {
    expect(eventToAccelerator(fakeEvent({ key: 'F1' }))).toBe('F1');
  });

  it('returns null for lone modifier keys', () => {
    expect(eventToAccelerator(fakeEvent({ key: 'Control' }))).toBeNull();
    expect(eventToAccelerator(fakeEvent({ key: 'Shift' }))).toBeNull();
    expect(eventToAccelerator(fakeEvent({ key: 'Alt' }))).toBeNull();
  });

  it('builds CTRL+key combo (Tauri uppercase format)', () => {
    expect(eventToAccelerator(fakeEvent({ key: 'a', ctrlKey: true }))).toBe('CTRL+A');
  });

  it('builds CTRL+SHIFT+key combo', () => {
    const acc = eventToAccelerator(
      fakeEvent({ key: 'F5', ctrlKey: true, shiftKey: true })
    );
    expect(acc).toBe('CTRL+SHIFT+F5');
  });

  it('replaces + with PLUS', () => {
    expect(eventToAccelerator(fakeEvent({ key: '+' }))).toBe('PLUS');
  });

  it('replaces space with SPACE', () => {
    expect(eventToAccelerator(fakeEvent({ key: ' ' }))).toBe('SPACE');
  });

  it('returns null for Meta key', () => {
    expect(eventToAccelerator(fakeEvent({ key: 'Meta' }))).toBeNull();
  });

  it('returns null for CapsLock', () => {
    expect(eventToAccelerator(fakeEvent({ key: 'CapsLock' }))).toBeNull();
  });

  it('returns null for NumLock and ScrollLock', () => {
    expect(eventToAccelerator(fakeEvent({ key: 'NumLock' }))).toBeNull();
    expect(eventToAccelerator(fakeEvent({ key: 'ScrollLock' }))).toBeNull();
  });

  it('builds ALT+key combo', () => {
    expect(eventToAccelerator(fakeEvent({ key: 'F2', altKey: true }))).toBe('ALT+F2');
  });

  it('builds CTRL+ALT+SHIFT triple-modifier combo', () => {
    expect(
      eventToAccelerator(fakeEvent({ key: 'Delete', ctrlKey: true, altKey: true, shiftKey: true }))
    ).toBe('CTRL+ALT+SHIFT+DELETE');
  });

  it('uppercases plain letter keys', () => {
    expect(eventToAccelerator(fakeEvent({ key: 'a' }))).toBe('A');
    expect(eventToAccelerator(fakeEvent({ key: 'z' }))).toBe('Z');
  });

  it('returns digit keys unchanged', () => {
    expect(eventToAccelerator(fakeEvent({ key: '5' }))).toBe('5');
  });
});
