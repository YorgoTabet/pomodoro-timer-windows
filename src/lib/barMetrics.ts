/**
 * Every horizontal measurement in the pill, derived from each other as in the Swift
 * `FloatingBar.Metrics`. The row has to add up exactly or the controls shift when
 * the box opens, and the compact width has to fall one trailing inset past the play
 * button.
 */
export const METRICS = {
  height: 46,
  leading: 11,
  trailing: 10,
  ring: 28,
  ringGap: 9,
  readout: 74,
  readoutGap: 12,
  primary: 28,
  secondary: 26,
  controlGap: 6,
  cornerRadius: 15,
} as const;

const M = METRICS;

/** The play button's leading edge, measured from the pill's own. */
export const primaryX = M.leading + M.ring + M.ringGap + M.readout + M.readoutGap;

/** The leading edge of the nth secondary control, counting outward from the play button. */
export const secondaryX = (index: number) =>
  primaryX + M.primary + M.controlGap + index * (M.secondary + M.controlGap);

/** How far back the nth secondary control sits when parked, so it lands under the play button. */
export const tuck = (index: number) => primaryX - secondaryX(index);

/** Two secondary controls and the trailing inset past them. */
export const width = secondaryX(1) + M.secondary + M.trailing;

/** The collapsed form ends one trailing inset past the play button. */
export const compactWidth = primaryX + M.primary + M.trailing;

/** How long the two forms take to swap, rounded up so a click never lands in the gap. */
export const MORPH_MS = 550;
