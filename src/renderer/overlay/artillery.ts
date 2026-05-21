import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AppSettings } from '../../shared/types';
import type {
  AuthStatus,
  Instance,
  Layer,
  Snapshot,
  Group,
  Battery,
  Target,
  Solution,
  SseStatus,
  ArtyEvent,
} from '../../shared/arty-types';

// ── Pure helpers (exported for tests) ────────────────────────────────────────

export function findBatteriesForGroup(
  snapshot: Snapshot,
  groupId: string | null
): Battery[] {
  if (!groupId) return [];
  return snapshot.groups.find((g) => g.id === groupId)?.batteries ?? [];
}

export function pickValidId<T extends { id: string }>(
  items: T[],
  desired: string | null
): string | null {
  if (desired && items.some((i) => i.id === desired)) return desired;
  return items[0]?.id ?? null;
}

/** Compute azimuth (deg, 0..360, North = 0) and distance (m) from battery to target. */
export function computeSolution(
  battery: { x: number; y: number },
  target: { x: number; y: number },
  range?: { min: number; max: number } | null
): Solution {
  const dx = target.x - battery.x;
  const dy = target.y - battery.y;
  const distance = Math.sqrt(dx * dx + dy * dy);
  // atan2(dx, -dy) yields 0° at North, increasing clockwise.
  let az = (Math.atan2(dx, -dy) * 180) / Math.PI;
  if (az < 0) az += 360;
  const inRange = range ? distance >= range.min && distance <= range.max : true;
  return {
    azimuth: az,
    distance,
    rangeMin: range?.min ?? null,
    rangeMax: range?.max ?? null,
    inRange,
  };
}

// ── State ────────────────────────────────────────────────────────────────────

interface ArtyState {
  settings: AppSettings;
  auth: AuthStatus;
  instances: Instance[];
  layers: Layer[];
  snapshot: Snapshot | null;
  status: SseStatus;
  unlisteners: UnlistenFn[];
}

const state: ArtyState = {
  settings: null as unknown as AppSettings,
  auth: { authenticated: false },
  instances: [],
  layers: [],
  snapshot: null,
  status: 'idle',
  unlisteners: [],
};

// ── DOM lookup ───────────────────────────────────────────────────────────────

function $(id: string): HTMLElement {
  const el = document.getElementById(id);
  if (!el) throw new Error(`Missing element: ${id}`);
  return el;
}

// ── Settings persistence ─────────────────────────────────────────────────────

async function persist(patch: Partial<AppSettings>): Promise<void> {
  Object.assign(state.settings, patch);
  await invoke('save_settings_cmd', { patch });
}

// ── Status indicator ─────────────────────────────────────────────────────────

function setStatus(status: SseStatus): void {
  state.status = status;
  const dot = $('arty-status');
  dot.classList.remove(
    'connecting',
    'connected',
    'reconnecting',
    'disconnected',
    'auth-required',
  );
  if (status === 'idle') {
    dot.classList.add('hidden');
    return;
  }
  dot.classList.remove('hidden');
  dot.classList.add(status);
  dot.title = `SSE: ${status}`;
}

// ── Render ───────────────────────────────────────────────────────────────────

function showError(msg: string | null, target: 'arty-error' | 'arty-auth-error' = 'arty-error'): void {
  const el = $(target);
  if (!msg) {
    el.classList.add('hidden');
    el.textContent = '';
    return;
  }
  el.classList.remove('hidden');
  el.textContent = msg;
}

function renderAuth(): void {
  const authBox = $('arty-auth');
  const mainBox = $('arty-main');
  if (state.auth.authenticated) {
    authBox.classList.add('hidden');
    mainBox.classList.remove('hidden');
    const user = $('arty-user');
    user.textContent = state.auth.pseudo
      ? `${state.auth.pseudo}${state.auth.role ? ` · ${state.auth.role}` : ''}`
      : 'Connecté';
  } else {
    authBox.classList.remove('hidden');
    mainBox.classList.add('hidden');
  }
}

function fillSelect(
  select: HTMLSelectElement,
  items: { id: string; name: string }[],
  selectedId: string | null,
  placeholder: string,
): void {
  select.innerHTML = '';
  const ph = document.createElement('option');
  ph.value = '';
  ph.textContent = placeholder;
  select.appendChild(ph);
  for (const it of items) {
    const opt = document.createElement('option');
    opt.value = it.id;
    opt.textContent = it.name;
    if (it.id === selectedId) opt.selected = true;
    select.appendChild(opt);
  }
  select.disabled = items.length === 0;
}

function renderSelectors(): void {
  fillSelect(
    $('arty-instance') as HTMLSelectElement,
    state.instances,
    state.settings.artyInstanceId,
    '— Instance —',
  );

  const snap = state.snapshot;
  const groups: Group[] = snap?.groups ?? [];
  fillSelect(
    $('arty-group') as HTMLSelectElement,
    groups,
    state.settings.artyGroupId,
    '— Groupe —',
  );

  const batteries = snap ? findBatteriesForGroup(snap, state.settings.artyGroupId) : [];
  fillSelect(
    $('arty-battery') as HTMLSelectElement,
    batteries,
    state.settings.artyBatteryId,
    '— Batterie —',
  );

  const rawTargets = snap
    ? (snap.groups.find((g) => g.id === state.settings.artyGroupId)?.targets ?? [])
    : [];
  const targets = rawTargets.map((t, i) => ({
    id: t.id,
    name: `Cible ${i + 1} (${Math.round(t.position.x)}, ${Math.round(t.position.y)})`,
  }));
  void targets; // target selector removed; focusedTargetId comes from snapshot
}

function renderSolution(): void {
  const box = $('arty-solution');
  const multiBox = $('arty-all-batteries');
  const snap = state.snapshot;
  const showAll = state.settings.artyShowAllBatteries;

  if (!snap) {
    box.classList.add('hidden');
    multiBox.classList.add('hidden');
    return;
  }

  const group = snap.groups.find((g) => g.id === state.settings.artyGroupId);
  const target = group?.targets.find((t) => t.id === group?.focusedTargetId);

  if (!target) {
    box.classList.add('hidden');
    multiBox.classList.add('hidden');
    return;
  }

  if (showAll) {
    box.classList.add('hidden');
    renderAllBatteries(group!, target);
    return;
  }

  multiBox.classList.add('hidden');
  const battery = group?.batteries.find((b) => b.id === state.settings.artyBatteryId);
  if (!battery?.position) {
    box.classList.add('hidden');
    return;
  }
  const sol = solveBatteryTarget(battery, target);
  box.classList.remove('hidden');
  $('arty-distance').textContent = `${sol.distance.toFixed(0)} m`;
  $('arty-azimuth').textContent = `${sol.angle.toFixed(1)}°`;
  $('arty-type').textContent = battery.type ?? '—';
  $('arty-out-of-range').classList.add('hidden');
}

/**
 * Returns the firing solution for a battery against a target.
 * Prefers the server-precomputed entry in `battery.solutions` (which
 * accounts for wind bias), falling back to a local azimuth/distance
 * calculation when no precomputed solution exists.
 */
export function solveBatteryTarget(
  battery: Battery,
  target: Target,
): { angle: number; distance: number; source: 'server' | 'local' } {
  const pre = battery.solutions?.find((s) => s.targetId === target.id);
  if (pre) {
    return { angle: pre.angle, distance: pre.distance, source: 'server' };
  }
  if (!battery.position) return { angle: 0, distance: 0, source: 'local' };
  const sol = computeSolution(battery.position, target.position);
  return { angle: sol.azimuth, distance: sol.distance, source: 'local' };
}

function renderAllBatteries(group: Group, target: Target): void {
  const box = $('arty-all-batteries');
  const tbody = $('arty-batteries-tbody');
  tbody.innerHTML = '';
  for (const battery of group.batteries) {
    const sol = solveBatteryTarget(battery, target);
    const tr = document.createElement('tr');
    if (battery.id === state.settings.artyBatteryId) tr.classList.add('active');
    const nameTd = document.createElement('td');
    nameTd.textContent = battery.type
      ? `${battery.name} (${battery.type})`
      : battery.name;
    const solTd = document.createElement('td');
    solTd.className = 'solution';
    const distSpan = document.createElement('span');
    distSpan.className = 'arty-dist';
    distSpan.textContent = `${sol.distance.toFixed(0)} m`;
    const sep = document.createElement('span');
    sep.className = 'arty-bullet';
    sep.textContent = '•';
    const azSpan = document.createElement('span');
    azSpan.className = 'arty-az';
    azSpan.textContent = `${sol.angle.toFixed(1)}°`;
    solTd.append(distSpan, sep, azSpan);
    tr.append(nameTd, solTd);
    tbody.appendChild(tr);
  }
  box.classList.remove('hidden');
}

function renderAll(): void {
  renderAuth();
  if (state.auth.authenticated) {
    renderSelectors();
    renderSolution();
  }
}

// ── Data loading ─────────────────────────────────────────────────────────────

async function refreshAuth(): Promise<void> {
  try {
    state.auth = await invoke<AuthStatus>('auth_status');
  } catch {
    state.auth = { authenticated: false };
  }
}

async function loadInstances(): Promise<void> {
  try {
    state.instances = await invoke<Instance[]>('arty_list_instances');
    showError(null);
  } catch (e) {
    state.instances = [];
    showError(String(e));
  }
}

async function loadLayers(instanceId: string): Promise<void> {
  try {
    state.layers = await invoke<Layer[]>('arty_list_layers', { instanceId });
    showError(null);
  } catch (e) {
    state.layers = [];
    showError(String(e));
  }
}

async function loadSnapshot(instanceId: string, layerId: string): Promise<void> {
  try {
    state.snapshot = await invoke<Snapshot>('arty_get_snapshot', { instanceId, layerId });
    showError(null);
  } catch (e) {
    state.snapshot = null;
    showError(String(e));
  }
}

// ── SSE lifecycle ────────────────────────────────────────────────────────────

async function connectSse(instanceId: string): Promise<void> {
  setStatus('connecting');
  try {
    await invoke('arty_sse_connect', { instanceId });
  } catch (e) {
    setStatus('disconnected');
    showError(String(e));
  }
}

async function disconnectSse(): Promise<void> {
  try {
    await invoke('arty_sse_disconnect');
  } catch {
    /* ignore */
  }
  setStatus('idle');
}

function applyArtyEvent(evt: ArtyEvent): void {
  // `instance.changed` signals any mutation on the instance data —
  // re-fetch the snapshot conservatively.
  if (evt.type !== 'instance.changed') return;
  const instanceId = state.settings.artyInstanceId;
  const layerId = state.settings.artyLayerId;
  if (!instanceId || !layerId) return;
  void loadSnapshot(instanceId, layerId).then(renderAll);
}

// ── Wiring ───────────────────────────────────────────────────────────────────

function bindSelectors(): void {
  ($('arty-instance') as HTMLSelectElement).addEventListener('change', async (e) => {
    const id = (e.target as HTMLSelectElement).value || null;
    await persist({
      artyInstanceId: id,
      artyLayerId: null,
      artyGroupId: null,
      artyBatteryId: null,
    });
    state.layers = [];
    state.snapshot = null;
    await disconnectSse();
    if (id) {
      await loadLayers(id);
      const lay = state.layers[0]?.id ?? null;
      if (lay) {
        await persist({ artyLayerId: lay });
        await loadSnapshot(id, lay);
      }
      await connectSse(id);
    }
    renderAll();
  });

  ($('arty-group') as HTMLSelectElement).addEventListener('change', async (e) => {
    const id = (e.target as HTMLSelectElement).value || null;
    await persist({ artyGroupId: id, artyBatteryId: null });
    renderAll();
  });

  ($('arty-battery') as HTMLSelectElement).addEventListener('change', async (e) => {
    const id = (e.target as HTMLSelectElement).value || null;
    await persist({ artyBatteryId: id });
    renderAll();
  });

  const showAll = $('arty-show-all') as HTMLInputElement;
  showAll.checked = state.settings.artyShowAllBatteries;
  showAll.addEventListener('change', async () => {
    await persist({ artyShowAllBatteries: showAll.checked });
    renderAll();
  });
}

function bindAuth(): void {
  $('arty-btn-discord').addEventListener('click', async () => {
    showError(null, 'arty-auth-error');
    try {
      await invoke('auth_discord_start');
    } catch (e) {
      showError(String(e), 'arty-auth-error');
    }
  });

  $('arty-btn-local').addEventListener('click', async () => {
    showError(null, 'arty-auth-error');
    const pseudo = ($('arty-pseudo') as HTMLInputElement).value.trim();
    const password = ($('arty-password') as HTMLInputElement).value;
    if (!pseudo || !password) {
      showError('Pseudo et mot de passe requis.', 'arty-auth-error');
      return;
    }
    try {
      await invoke('auth_local_login', { pseudo, password });
    } catch (e) {
      showError(String(e), 'arty-auth-error');
    }
  });

  $('arty-btn-logout').addEventListener('click', async () => {
    try {
      await invoke('auth_logout');
    } catch {
      /* ignore */
    }
    state.auth = { authenticated: false };
    state.instances = [];
    state.layers = [];
    state.snapshot = null;
    setStatus('idle');
    renderAll();
  });
}

// ── Public API ───────────────────────────────────────────────────────────────

export async function initArtillery(settings: AppSettings): Promise<void> {
  state.settings = settings;
  setStatus('idle');

  bindSelectors();
  bindAuth();

  // Tauri events.
  state.unlisteners.push(
    await listen('auth-success', async () => {
      await refreshAuth();
      await loadInstances();
      if (state.settings.artyInstanceId) {
        await loadLayers(state.settings.artyInstanceId);
        const lay = state.layers[0]?.id ?? null;
        if (lay) {
          await persist({ artyLayerId: lay });
          await loadSnapshot(state.settings.artyInstanceId, lay);
        }
        await connectSse(state.settings.artyInstanceId);
      }
      renderAll();
    }),
    await listen('auth-required', async () => {
      state.auth = { authenticated: false };
      setStatus('auth-required');
      renderAll();
    }),
    await listen<string>('arty-sse-status', (e) => {
      setStatus(e.payload as SseStatus);
    }),
    await listen<ArtyEvent>('arty-sse', (e) => {
      applyArtyEvent(e.payload);
    }),
  );

  await refreshAuth();
  if (state.auth.authenticated) {
    await loadInstances();
    const inst = pickValidId(state.instances, state.settings.artyInstanceId);
    if (inst) {
      await persist({ artyInstanceId: inst });
      await loadLayers(inst);
      const lay = state.layers[0]?.id ?? null;
      if (lay) {
        await persist({ artyLayerId: lay });
        await loadSnapshot(inst, lay);
      }
      await connectSse(inst);
    }
  }
  renderAll();
}

export async function showArtillery(): Promise<void> {
  $('arty-panel').classList.remove('hidden');
}

export async function hideArtillery(): Promise<void> {
  $('arty-panel').classList.add('hidden');
}
