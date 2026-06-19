import type { QuorumResponse } from "./types.js";

/** Tracks finalized quorum ticks and helps clients pick consistent read ticks. */
export class QuorumTracker {
  private latestQuorumTick = 0;

  update(tick: number, finalized: boolean) {
    if (finalized && tick > this.latestQuorumTick) {
      this.latestQuorumTick = tick;
    }
  }

  get latest(): number {
    return this.latestQuorumTick;
  }

  /** Build a query string snippet for the current latest quorum tick. */
  query(): string {
    if (this.latestQuorumTick <= 0) return "";
    return `?min_tick=${this.latestQuorumTick}`;
  }
}

export function isFinalized(response: QuorumResponse): boolean {
  return response.finalized;
}

export function parseQuorumResponse(response: QuorumResponse): {
  finalized: boolean;
  tick: number;
  stake?: bigint;
  threshold?: bigint;
  roots?: QuorumResponse["roots"];
} {
  return {
    finalized: response.finalized,
    tick: response.tick,
    stake: response.stake ? BigInt(response.stake) : undefined,
    threshold: response.threshold ? BigInt(response.threshold) : undefined,
    roots: response.roots,
  };
}
