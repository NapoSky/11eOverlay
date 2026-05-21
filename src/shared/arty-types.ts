// Wire types mirroring src-tauri/src/api/types.rs (camelCase).

export interface AuthStatus {
  authenticated: boolean;
  pseudo?: string | null;
  role?: string | null;
}

export interface Instance {
  id: string;
  name: string;
  ownerAccountId: string;
  createdAt: string;
}

export interface Layer {
  id: string;
  instanceId: string;
  name: string;
  type: string;
  createdAt: string;
  updatedAt: string;
}

export interface Group {
  id: string;
  name: string;
  focusedTargetId: string | null;
  batteries: Battery[];
  targets: Target[];
}

export interface Battery {
  id: string;
  name: string;
  type?: string;
  groupId: string;
  position: Position | null;
  solutions?: BatterySolution[];
}

export interface BatterySolution {
  targetId: string;
  distance: number;
  angle: number;
  windBias?: Position;
}

export interface Target {
  id: string;
  groupId: string;
  type?: string;
  position: Position;
  windForce: number;
  windDirection: number;
}

export interface Position {
  x: number;
  y: number;
}

export interface Solution {
  azimuth: number;
  distance: number;
  rangeMin?: number | null;
  rangeMax?: number | null;
  inRange: boolean;
}

export interface Snapshot {
  layerId: string;
  groups: Group[];
}

/** Server-Sent Event payload as forwarded by the Rust backend. */
export interface ArtyEvent {
  type: string;
  data: unknown;
}

export type SseStatus =
  | 'idle'
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'disconnected'
  | 'auth-required';

export type AppMode = 'lexique' | 'artillerie';
