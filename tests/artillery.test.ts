import { describe, it, expect } from 'vitest';
import {
  findBatteriesForGroup,
  pickValidId,
  computeSolution,
} from '../src/renderer/overlay/artillery';
import type { Snapshot } from '../src/shared/arty-types';

const snap: Snapshot = {
  layerId: 'l1',
  groups: [
    {
      id: 'g1',
      name: 'Group 1',
      batteries: [
        { id: 'b1', name: 'B1', groupId: 'g1', position: { x: 0, y: 0 } },
        { id: 'b2', name: 'B2', groupId: 'g1', position: { x: 10, y: 0 } },
      ],
      targets: [
        { id: 't1', groupId: 'g1', position: { x: 0, y: 100 }, windForce: 0, windDirection: 0 },
      ],
    },
    {
      id: 'g2',
      name: 'Group 2',
      batteries: [
        { id: 'b3', name: 'B3', groupId: 'g2', position: { x: 0, y: 10 } },
      ],
      targets: [],
    },
  ],
};

describe('findBatteriesForGroup', () => {
  it('returns only batteries of the given group', () => {
    const out = findBatteriesForGroup(snap, 'g1');
    expect(out.map((b) => b.id)).toEqual(['b1', 'b2']);
  });

  it('returns empty list when groupId is null', () => {
    expect(findBatteriesForGroup(snap, null)).toEqual([]);
  });

  it('returns empty list when group has no batteries', () => {
    expect(findBatteriesForGroup(snap, 'gX')).toEqual([]);
  });
});

describe('pickValidId', () => {
  const items = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];

  it('keeps the desired id when present', () => {
    expect(pickValidId(items, 'b')).toBe('b');
  });

  it('falls back to first when desired is missing', () => {
    expect(pickValidId(items, 'z')).toBe('a');
  });

  it('falls back to first when desired is null', () => {
    expect(pickValidId(items, null)).toBe('a');
  });

  it('returns null on empty list', () => {
    expect(pickValidId([], 'a')).toBeNull();
  });
});

describe('computeSolution', () => {
  it('returns azimuth 0° due north (positive -Y)', () => {
    const sol = computeSolution({ x: 0, y: 100 }, { x: 0, y: 0 });
    expect(sol.azimuth).toBeCloseTo(0, 5);
    expect(sol.distance).toBeCloseTo(100, 5);
  });

  it('returns azimuth 90° due east', () => {
    const sol = computeSolution({ x: 0, y: 0 }, { x: 100, y: 0 });
    expect(sol.azimuth).toBeCloseTo(90, 5);
    expect(sol.distance).toBeCloseTo(100, 5);
  });

  it('returns azimuth 180° due south', () => {
    const sol = computeSolution({ x: 0, y: 0 }, { x: 0, y: 100 });
    expect(sol.azimuth).toBeCloseTo(180, 5);
  });

  it('returns azimuth 270° due west', () => {
    const sol = computeSolution({ x: 100, y: 0 }, { x: 0, y: 0 });
    expect(sol.azimuth).toBeCloseTo(270, 5);
  });

  it('reports inRange=true when no range given', () => {
    const sol = computeSolution({ x: 0, y: 0 }, { x: 50, y: 0 });
    expect(sol.inRange).toBe(true);
  });

  it('reports inRange=false when distance below min', () => {
    const sol = computeSolution(
      { x: 0, y: 0 },
      { x: 50, y: 0 },
      { min: 100, max: 200 },
    );
    expect(sol.inRange).toBe(false);
  });

  it('reports inRange=false when distance above max', () => {
    const sol = computeSolution(
      { x: 0, y: 0 },
      { x: 500, y: 0 },
      { min: 100, max: 200 },
    );
    expect(sol.inRange).toBe(false);
  });

  it('reports inRange=true when distance within bounds', () => {
    const sol = computeSolution(
      { x: 0, y: 0 },
      { x: 150, y: 0 },
      { min: 100, max: 200 },
    );
    expect(sol.inRange).toBe(true);
  });
});
