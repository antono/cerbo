import { describe, expect, it } from 'vitest';
import { ZOOM_LEVELS, ZOOM_MAX, ZOOM_MIN, clampZoom, parseZoom, zoomIn, zoomOut, zoomPercent } from './zoom';

describe('zoom', () => {
  it('clamps to 50%..200% and treats junk as 100%', () => {
    expect(clampZoom(1.37)).toBe(1.37);
    expect(clampZoom(0.1)).toBe(ZOOM_MIN);
    expect(clampZoom(13.7)).toBe(ZOOM_MAX);
    expect(clampZoom(Number.NaN)).toBe(1);
    expect(clampZoom(Infinity)).toBe(1);
    expect(clampZoom(0)).toBe(1);
    expect(clampZoom(-1.37)).toBe(1);
  });

  it('reads a stored value, falling back to 100%', () => {
    expect(parseZoom(1.25)).toBe(1.25);
    expect(parseZoom(137)).toBe(ZOOM_MAX);
    expect(parseZoom(null)).toBe(1);
    expect(parseZoom(undefined)).toBe(1);
    expect(parseZoom('1.5')).toBe(1);
  });

  it('steps through the levels in both directions', () => {
    let z = 1;
    const up: number[] = [];
    while (z < ZOOM_MAX) up.push((z = zoomIn(z)));
    expect(up).toEqual([1.1, 1.25, 1.5, 1.75, 2]);

    z = 1;
    const down: number[] = [];
    while (z > ZOOM_MIN) down.push((z = zoomOut(z)));
    expect(down).toEqual([0.9, 0.8, 0.75, 0.67, 0.5]);
  });

  it('stays at the limits', () => {
    expect(zoomIn(ZOOM_MAX)).toBe(ZOOM_MAX);
    expect(zoomOut(ZOOM_MIN)).toBe(ZOOM_MIN);
    expect(zoomIn(13.7)).toBe(ZOOM_MAX);
    expect(zoomOut(0.137)).toBe(ZOOM_MIN);
  });

  it('snaps a value between levels to the next level', () => {
    expect(zoomIn(1.37)).toBe(1.5);
    expect(zoomOut(1.37)).toBe(1.25);
    expect(zoomIn(1.0000001)).toBe(1.1);
    expect(zoomOut(0.9999999)).toBe(0.9);
  });

  it('keeps the levels sorted within the limits', () => {
    expect([...ZOOM_LEVELS].sort((a, b) => a - b)).toEqual(ZOOM_LEVELS);
    expect(ZOOM_LEVELS[0]).toBe(ZOOM_MIN);
    expect(ZOOM_LEVELS.at(-1)).toBe(ZOOM_MAX);
  });

  it('formats percent', () => {
    expect(zoomPercent(1)).toBe('100%');
    expect(zoomPercent(0.67)).toBe('67%');
    expect(zoomPercent(1.37)).toBe('137%');
  });
});
