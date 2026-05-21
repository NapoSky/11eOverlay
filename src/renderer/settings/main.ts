import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { AppSettings } from '../../shared/types';

// ── Hotkey capture ────────────────────────────────────────────────────────────

const IGNORED_KEYS = new Set([
  'Control', 'Shift', 'Alt', 'Meta',
  'CapsLock', 'NumLock', 'ScrollLock'
]);

export function eventToAccelerator(e: KeyboardEvent): string | null {
  if (IGNORED_KEYS.has(e.key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey)  parts.push('CTRL');
  if (e.altKey)   parts.push('ALT');
  if (e.shiftKey) parts.push('SHIFT');
  const key = e.key === '+' ? 'PLUS' : e.key === ' ' ? 'SPACE' : e.key.toUpperCase();
  parts.push(key);
  return parts.join('+');
}

function captureHotkey(input: HTMLInputElement, onCapture: (acc: string) => void): void {
  input.value = '…';
  const onKey = (e: KeyboardEvent): void => {
    e.preventDefault();
    const acc = eventToAccelerator(e);
    if (!acc) return;
    input.value = acc;
    onCapture(acc);
    document.removeEventListener('keydown', onKey);
  };
  document.addEventListener('keydown', onKey);
}

// ── Bootstrap ─────────────────────────────────────────────────────────────────

async function main(): Promise<void> {
  const settings = await invoke<AppSettings>('get_settings');

  const inputHotkey          = document.getElementById('input-hotkey')          as HTMLInputElement;
  const inputOpacity          = document.getElementById('input-opacity')          as HTMLInputElement;
  const opacityValue          = document.getElementById('opacity-value')!;
  const inputFontsize         = document.getElementById('input-fontsize')         as HTMLInputElement;
  const fontsizeValue         = document.getElementById('fontsize-value')!;

  inputHotkey.value          = settings.hotkey;
  inputOpacity.value          = String(Math.round(settings.opacity * 100));
  opacityValue.textContent    = `${inputOpacity.value}%`;
  inputFontsize.value         = String(settings.fontSize);
  fontsizeValue.textContent   = `${inputFontsize.value}px`;

  inputHotkey.addEventListener('click', () => {
    captureHotkey(inputHotkey, () => {});
  });

  inputOpacity.addEventListener('input', () => {
    opacityValue.textContent = `${inputOpacity.value}%`;
  });

  inputFontsize.addEventListener('input', () => {
    fontsizeValue.textContent = `${inputFontsize.value}px`;
  });

  document.getElementById('btn-save')!.addEventListener('click', async () => {
    await invoke('save_settings_cmd', {
      patch: {
        hotkey:          inputHotkey.value,
        opacity:         Number(inputOpacity.value) / 100,
        fontSize:        Number(inputFontsize.value),
      }
    });
    getCurrentWindow().close();
  });

  document.getElementById('btn-cancel')!.addEventListener('click', () =>
    getCurrentWindow().close()
  );

  document.getElementById('btn-close')!.addEventListener('click', () =>
    getCurrentWindow().close()
  );
}

if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  main();
}
