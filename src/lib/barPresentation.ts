/** Which of the floating bar's two forms is on screen. */
export type BarMode = "expanded" | "compact";

export interface BarPolicy {
  compactEnabled: boolean;
  isRunning: boolean;
  hovering: boolean;
  /** Set by every phase change, cleared once the user has looked at the bar or the backstop expires. */
  awaitingAcknowledgement: boolean;
}

export const DEFAULT_POLICY: BarPolicy = {
  compactEnabled: false,
  isRunning: false,
  hovering: false,
  awaitingAcknowledgement: false,
};

/**
 * The whole compact-mode decision. Four vetoes, none of them ordered: the bar is
 * compact only when none of them applies. The Mac app has a fifth for VoiceOver,
 * which has no web equivalent worth wiring here.
 */
export function barMode(p: BarPolicy): BarMode {
  if (!p.compactEnabled || !p.isRunning || p.hovering || p.awaitingAcknowledgement) {
    return "expanded";
  }
  return "compact";
}

/** How long an unacknowledged phase change holds the bar open. */
export const ACKNOWLEDGEMENT_BACKSTOP_MS = 30_000;

/** Holds the policy and the one clock that gives up waiting for the user to notice a phase change. */
export class BarPresentationModel {
  policy: BarPolicy = { ...DEFAULT_POLICY };
  private backstop: ReturnType<typeof setTimeout> | undefined;
  // Leaving alone must not clear an acknowledgement nobody saw: the rule is enter, then leave.
  private sawHoverSincePhaseChange = false;

  constructor(
    private readonly onChange: (policy: BarPolicy) => void = () => {},
    private readonly backstopMs = ACKNOWLEDGEMENT_BACKSTOP_MS,
  ) {}

  get mode(): BarMode {
    return barMode(this.policy);
  }

  setCompactEnabled(compactEnabled: boolean) {
    this.update({ compactEnabled });
  }

  setRunning(isRunning: boolean) {
    this.update({ isRunning });
  }

  /** Applied the instant it arrives; the width spring absorbs a cursor that only brushes past. */
  setHovering(hovering: boolean) {
    if (hovering === this.policy.hovering) return;
    if (hovering) {
      this.sawHoverSincePhaseChange = true;
    } else if (this.sawHoverSincePhaseChange) {
      this.clearAcknowledgement();
    }
    this.update({ hovering });
  }

  /** A phase elapsed: show the full bar and hold it until it has been seen. */
  beginPhaseChange() {
    this.sawHoverSincePhaseChange = false;
    clearTimeout(this.backstop);
    this.backstop = setTimeout(() => this.clearAcknowledgement(), this.backstopMs);
    this.update({ awaitingAcknowledgement: true });
  }

  dispose() {
    clearTimeout(this.backstop);
  }

  private clearAcknowledgement() {
    clearTimeout(this.backstop);
    this.backstop = undefined;
    this.update({ awaitingAcknowledgement: false });
  }

  private update(patch: Partial<BarPolicy>) {
    this.policy = { ...this.policy, ...patch };
    this.onChange(this.policy);
  }
}
