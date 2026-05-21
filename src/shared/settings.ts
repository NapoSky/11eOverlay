import type { AppSettings } from './types';

export const DEFAULT_SETTINGS: Readonly<AppSettings> = {
  hotkey:         'F1',
  opacity: 0.88,
  width:   320,
  height:  520,
  x:       100,
  y:       100,
  fontSize: 12,
  activeSection: null,
  activeMode:    'lexique',
  artyInstanceId: null,
  artyLayerId:    null,
  artyGroupId:    null,
  artyBatteryId:  null,
  artyShowAllBatteries: false,
};

/** Non-destructive merge: only keys present in patch override current. */
export function mergeSettings(
  current: AppSettings,
  patch: Partial<AppSettings>
): AppSettings {
  return { ...current, ...patch };
}
