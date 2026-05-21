import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { AppSettings, ContentSection, ContentItem } from '../../shared/types';
import { initArtillery } from './artillery';

// ── Pure helpers (exported for tests) ────────────────────────────────────────

export function filterSections(
  sections: ContentSection[],
  query: string
): ContentSection[] {
  if (!query) return sections;
  const q = query.toLowerCase();
  return sections
    .map((sec) => ({
      ...sec,
      items: sec.items.filter(
        (item: ContentItem) =>
          item.key.toLowerCase().includes(q) ||
          item.value.toLowerCase().includes(q)
      )
    }))
    .filter((sec) => sec.items.length > 0);
}

export function highlight(text: string, query: string): string {
  if (!query) return escapeHtml(text);
  const escaped = escapeHtml(text);
  const q = escapeHtml(query);
  return escaped.replace(
    new RegExp(q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi'),
    (m) => `<mark>${m}</mark>`
  );
}

export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export function applyOpacity(opacity: number): void {
  document.documentElement.style.setProperty('--opacity', String(opacity));
}

export function applyFontSize(fontSize: number): void {
  document.documentElement.style.setProperty('--font-size', `${fontSize}px`);
}

// ── State ─────────────────────────────────────────────────────────────────────

let allSections: ContentSection[] = [];
let currentSettings: AppSettings;
let activeSection: string | null = null;
let pinnedItem: ContentItem | null = null;

// ── Render ────────────────────────────────────────────────────────────────────

function getFilteredSections(): ContentSection[] {
  const q = (document.getElementById('search-input') as HTMLInputElement).value.trim();
  let sections = filterSections(allSections, q);
  if (activeSection !== null) {
    sections = sections.filter((s) => s.section === activeSection);
  }
  return sections;
}

function renderList(sections: ContentSection[], query: string): void {
  const container = document.getElementById('content-list')!;
  container.innerHTML = '';

  if (sections.length === 0) {
    const empty = document.createElement('div');
    empty.style.cssText = 'color:#606070;padding:12px 8px;font-style:italic';
    empty.textContent = 'Aucun résultat.';
    container.appendChild(empty);
    updateScrollIndicator();
    return;
  }

  for (const sec of sections) {
    const block = document.createElement('div');
    block.className = 'section-block';

    const header = document.createElement('div');
    header.className = 'section-header';
    header.textContent = sec.section;
    block.appendChild(header);

    for (const item of sec.items) {
      const row = document.createElement('div');
      row.className = 'item-row';
      row.innerHTML =
        `<span class="item-key">${highlight(item.key, query)}</span>` +
        `<span class="item-arrow">→</span>` +
        `<span class="item-value">${highlight(item.value, query)}</span>`;

      row.addEventListener('dblclick', () => {
        const isSame = pinnedItem?.key === item.key && pinnedItem?.value === item.value;
        pinnedItem = isSame ? null : { key: item.key, value: item.value };
        renderPinnedItem();
      });

      block.appendChild(row);
    }

    container.appendChild(block);
  }

  updateScrollIndicator();
}

function renderContent(): void {
  const sections = getFilteredSections();
  const q = (document.getElementById('search-input') as HTMLInputElement).value.trim();
  renderList(sections, q);
}

// ── Chips ─────────────────────────────────────────────────────────────────────

function renderChips(): void {
  const container = document.getElementById('section-chips')!;
  container.innerHTML = '';

  const allBtn = document.createElement('button');
  allBtn.className = 'chip' + (activeSection === null ? ' active' : '');
  allBtn.textContent = 'Tout';
  allBtn.addEventListener('click', () => {
    activeSection = null;
    currentSettings.activeSection = null;
    void invoke('save_settings_cmd', { patch: { activeSection: null } });
    renderChips();
    renderContent();
  });
  container.appendChild(allBtn);

  for (const sec of allSections) {
    const btn = document.createElement('button');
    btn.className = 'chip' + (activeSection === sec.section ? ' active' : '');
    btn.textContent = sec.section;
    btn.addEventListener('click', () => {
      activeSection = activeSection === sec.section ? null : sec.section;
      currentSettings.activeSection = activeSection;
      void invoke('save_settings_cmd', { patch: { activeSection } });
      renderChips();
      renderContent();
    });
    container.appendChild(btn);
  }
}

// ── Pinned item ───────────────────────────────────────────────────────────────

function renderPinnedItem(): void {
  const container = document.getElementById('pinned-item')!;
  if (!pinnedItem) {
    container.classList.add('hidden');
    container.innerHTML = '';
    return;
  }
  container.classList.remove('hidden');
  container.innerHTML =
    `<div class="item-row pinned-row">` +
    `<span class="pin-mark">⊕</span>` +
    `<span class="item-key">${escapeHtml(pinnedItem.key)}</span>` +
    `<span class="item-arrow">→</span>` +
    `<span class="item-value">${escapeHtml(pinnedItem.value)}</span>` +
    `</div>`;
  container.querySelector('.pinned-row')!.addEventListener('dblclick', () => {
    pinnedItem = null;
    renderPinnedItem();
  });
}

// ── Scroll indicator ──────────────────────────────────────────────────────────

function updateScrollIndicator(): void {
  const list = document.getElementById('content-list')!;
  const fade = document.getElementById('scroll-fade')!;
  const hasMore = list.scrollHeight > list.clientHeight + list.scrollTop + 4;
  fade.classList.toggle('visible', hasMore);
}

// ── Dirty indicator ───────────────────────────────────────────────────────────

function markDirty(dirty: boolean): void {
  document.getElementById('btn-settings')!.classList.toggle('dirty', dirty);
}

// ── Settings panel ────────────────────────────────────────────────────────────

const IGNORED_KEYS = new Set([
  'Control', 'Shift', 'Alt', 'Meta',
  'CapsLock', 'NumLock', 'ScrollLock'
]);

function eventToAccelerator(e: KeyboardEvent): string | null {
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

function initSettingsPanel(): void {
  const panel               = document.getElementById('settings-panel')!;
  const hotkeyInput         = document.getElementById('settings-hotkey')          as HTMLInputElement;
  const opacityInput        = document.getElementById('settings-opacity')          as HTMLInputElement;
  const opacityValue        = document.getElementById('settings-opacity-value')!;
  const fontsizeInput       = document.getElementById('settings-fontsize')         as HTMLInputElement;
  const fontsizeValue       = document.getElementById('settings-fontsize-value')!;
  const btnSettings         = document.getElementById('btn-settings')!;
  const btnSave             = document.getElementById('settings-save')!;
  const btnCancel           = document.getElementById('settings-cancel')!;

  let pendingHotkey         = currentSettings.hotkey;

  function openPanel(): void {
    pendingHotkey          = currentSettings.hotkey;
    hotkeyInput.value          = pendingHotkey;
    opacityInput.value          = String(Math.round(currentSettings.opacity * 100));
    opacityValue.textContent    = `${opacityInput.value}%`;
    fontsizeInput.value         = String(currentSettings.fontSize);
    fontsizeValue.textContent   = `${fontsizeInput.value}px`;
    markDirty(false);
    panel.classList.remove('hidden');
  }

  function closePanel(): void {
    panel.classList.add('hidden');
    markDirty(false);
  }

  btnSettings.addEventListener('click', () => {
    panel.classList.contains('hidden') ? openPanel() : closePanel();
  });

  hotkeyInput.addEventListener('click', () => {
    captureHotkey(hotkeyInput, (acc) => {
      pendingHotkey = acc;
      markDirty(true);
    });
  });

  opacityInput.addEventListener('input', () => {
    opacityValue.textContent = `${opacityInput.value}%`;
    applyOpacity(Number(opacityInput.value) / 100);
    markDirty(true);
  });

  fontsizeInput.addEventListener('input', () => {
    fontsizeValue.textContent = `${fontsizeInput.value}px`;
    applyFontSize(Number(fontsizeInput.value));
    markDirty(true);
  });

  btnSave.addEventListener('click', async () => {
    const opacity  = Number(opacityInput.value) / 100;
    const fontSize = Number(fontsizeInput.value);
    await invoke('save_settings_cmd', {
      patch: {
        hotkey:          hotkeyInput.value,
        opacity,
        fontSize,
      }
    });
    currentSettings.hotkey          = hotkeyInput.value;
    currentSettings.opacity          = opacity;
    currentSettings.fontSize         = fontSize;
    applyOpacity(opacity);
    applyFontSize(fontSize);
    closePanel();
  });

  btnCancel.addEventListener('click', () => {
    pendingHotkey          = currentSettings.hotkey;
    applyOpacity(currentSettings.opacity);
    applyFontSize(currentSettings.fontSize);
    closePanel();
  });
}

// ── Bootstrap ────────────────────────────────────────────────────────────────

function initModeBar(): void {
  const lexiqueBtn = document.getElementById('mode-lexique')!;
  const artyBtn = document.getElementById('mode-artillerie')!;
  const lexiqueEls = [
    document.getElementById('search-bar')!,
    document.getElementById('section-chips')!,
    document.getElementById('pinned-item')!,
    document.getElementById('content-wrapper')!,
  ];
  const artyPanel = document.getElementById('arty-panel')!;

  function applyMode(mode: string): void {
    const arty = mode === 'artillerie';
    artyBtn.classList.toggle('active', arty);
    lexiqueBtn.classList.toggle('active', !arty);
    artyPanel.classList.toggle('hidden', !arty);
    for (const el of lexiqueEls) {
      el.classList.toggle('hidden', arty);
    }
  }

  applyMode(currentSettings.activeMode);

  lexiqueBtn.addEventListener('click', () => {
    if (currentSettings.activeMode === 'lexique') return;
    currentSettings.activeMode = 'lexique';
    applyMode('lexique');
    void invoke('save_settings_cmd', { patch: { activeMode: 'lexique' } });
  });

  artyBtn.addEventListener('click', () => {
    if (currentSettings.activeMode === 'artillerie') return;
    currentSettings.activeMode = 'artillerie';
    applyMode('artillerie');
    void invoke('save_settings_cmd', { patch: { activeMode: 'artillerie' } });
  });
}

async function main(): Promise<void> {
  const [settings, sections] = await Promise.all([
    invoke<AppSettings>('get_settings'),
    invoke<ContentSection[]>('get_content')
  ]);

  currentSettings = settings;
  allSections     = sections;
  activeSection   = settings.activeSection ?? null;

  applyOpacity(settings.opacity);
  applyFontSize(settings.fontSize);

  initSettingsPanel();
  renderChips();
  renderContent();

  // Initialise artillery module (loads auth, instances, snapshot if applicable).
  await initArtillery(currentSettings);
  initModeBar();

  const searchInput = document.getElementById('search-input') as HTMLInputElement;
  searchInput.addEventListener('input', renderContent);

  document.getElementById('content-list')!.addEventListener('scroll', updateScrollIndicator);

  // Auto-focus search on load and each time the overlay window regains focus
  searchInput.focus();
  window.addEventListener('focus', () => searchInput.focus());

  // Escape clears search when the input is focused
  searchInput.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      searchInput.value = '';
      renderContent();
    }
  });

  document.getElementById('btn-hide')!.addEventListener('click', () =>
    invoke('hide_overlay_cmd')
  );

  document.getElementById('btn-close')!.addEventListener('click', () =>
    invoke('quit_app')
  );

  // Sync settings changes from the standalone settings window
  await listen<AppSettings>('settings-updated', (event) => {
    currentSettings = { ...currentSettings, ...event.payload };
    applyOpacity(event.payload.opacity);
    applyFontSize(event.payload.fontSize);
  });
}

if (typeof window !== 'undefined' && typeof document !== 'undefined') {
  main();
}
