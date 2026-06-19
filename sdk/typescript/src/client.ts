import type {
  AccountId,
  BalanceResponse,
  CommutativeShift,
  FluidicClientOptions,
  OperatorInfo,
  QuorumResponse,
  ShiftStatusResponse,
  StateResponse,
  StateSnapshot,
  StatefulShift,
  TxHash,
} from "./types.js";
import { QuorumTracker } from "./quorum.js";
import { sleep } from "./utils.js";

export interface SubmitResult {
  hash: TxHash;
  status: "queued";
}

export class FluidicClient {
  readonly apiUrl: string;
  readonly wsUrl: string;
  readonly minTickMode: FluidicClientOptions["minTick"];
  readonly timeoutMs: number;
  readonly retries: number;
  readonly quorumTracker: QuorumTracker;

  private ws: WebSocket | null = null;
  private snapshotListeners: ((snap: StateSnapshot) => void)[] = [];
  private latestLocalTick = 0;

  constructor(options: FluidicClientOptions) {
    this.apiUrl = options.apiUrl.replace(/\/$/, "");
    this.wsUrl =
      options.wsUrl ??
      this.apiUrl.replace(/^http/, (m) => (m === "https" ? "wss" : "ws")) +
        "/api/ws";
    this.minTickMode = options.minTick ?? "none";
    this.timeoutMs = options.timeoutMs ?? 10_000;
    this.retries = options.retries ?? 3;
    this.quorumTracker = new QuorumTracker();
  }

  private minTickQuery(): string {
    if (typeof this.minTickMode === "number") {
      return `?min_tick=${this.minTickMode}`;
    }
    if (this.minTickMode === "quorum") {
      return this.quorumTracker.query();
    }
    if (this.minTickMode === "latest") {
      if (this.latestLocalTick > 0) {
        return `?min_tick=${this.latestLocalTick}`;
      }
    }
    return "";
  }

  private async fetchJson<T>(
    path: string,
    options: RequestInit = {},
    requireMinTick = true
  ): Promise<T> {
    const query = requireMinTick ? this.minTickQuery() : "";
    const url = `${this.apiUrl}${path}${query}`;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);

    let lastErr: unknown;
    for (let attempt = 0; attempt <= this.retries; attempt++) {
      try {
        const res = await fetch(url, {
          ...options,
          signal: controller.signal,
        });
        clearTimeout(timer);
        if (!res.ok) {
          const text = await res.text().catch(() => "");
          throw new Error(`HTTP ${res.status}: ${text}`);
        }
        return (await res.json()) as T;
      } catch (err) {
        lastErr = err;
        if (attempt < this.retries) await sleep(250 * (attempt + 1));
      }
    }
    throw lastErr;
  }

  private async postJson<T>(path: string, body: unknown): Promise<T> {
    return this.fetchJson<T>(
      path,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      },
      false
    );
  }

  /** Fetch global pool state. */
  async state(): Promise<StateResponse> {
    return this.fetchJson<StateResponse>("/api/state");
  }

  /** Fetch WAVE/USDC balances for a Fluidic account. */
  async balance(account: AccountId): Promise<BalanceResponse> {
    return this.fetchJson<BalanceResponse>(`/api/account/${account}/balance`);
  }

  /** Fetch the status of a submitted shift. */
  async shiftStatus(hash: TxHash): Promise<ShiftStatusResponse> {
    return this.fetchJson<ShiftStatusResponse>(`/api/shift/${hash}/status`);
  }

  /** Submit a commutative shift. */
  async submitCommutative(shift: CommutativeShift): Promise<SubmitResult> {
    return this.postJson<SubmitResult>("/api/shift/commutative", {
      coordinate: {
        components: shift.coordinate.components.map((c) => Number(c)),
      },
      delta: shift.delta.toString(),
      pool_id: shift.pool_id,
      nonce: Number(shift.nonce),
      timestamp_ns: Number(shift.timestamp_ns),
      signature: shift.signature,
    });
  }

  /** Submit a stateful shift. */
  async submitStateful(shift: StatefulShift): Promise<SubmitResult> {
    return this.postJson<SubmitResult>("/api/shift/stateful", {
      from: shift.from,
      to: shift.to,
      amount: shift.amount.toString(),
      vector_clock: {
        entries: Object.fromEntries(
          Object.entries(shift.vector_clock.entries).map(([k, v]) => [
            k,
            Number(v),
          ])
        ),
      },
      predecessors: shift.predecessors,
      nonce: Number(shift.nonce),
      timestamp_ns: Number(shift.timestamp_ns),
      signature: shift.signature,
    });
  }

  /** Register an account via the API faucet. */
  async register(publicKey: string): Promise<{
    account_id: AccountId;
    wave_account: string;
    usdc_account: string;
  }> {
    return this.postJson("/api/account/register", { public_key_hex: publicKey });
  }

  /** Fetch operator info for the node we are connected to. */
  async operatorInfo(): Promise<OperatorInfo> {
    return this.fetchJson<OperatorInfo>("/api/operator/info");
  }

  /** Fetch the list of currently staked operators. */
  async stakedOperators(): Promise<{ operators: OperatorInfo[] }> {
    return this.fetchJson("/api/operators");
  }

  /** Fetch the synthesis certificate for a tick, if any. */
  async certificate(tick: number): Promise<unknown | null> {
    try {
      return await this.fetchJson<unknown>(`/api/certificate/${tick}`);
    } catch (e) {
      if (e instanceof Error && e.message.includes("HTTP 404")) return null;
      throw e;
    }
  }

  /** Fetch quorum status for a tick. */
  async getQuorum(tick: number): Promise<QuorumResponse> {
    const res = await this.fetchJson<QuorumResponse>(`/api/quorum/${tick}`);
    this.quorumTracker.update(tick, res.finalized);
    if (res.finalized && tick > this.latestLocalTick) {
      this.latestLocalTick = tick;
    }
    return res;
  }

  /** Poll a shift until it is finalized (or rejected). */
  async waitForFinalization(
    hash: TxHash,
    options: { pollMs?: number; timeoutMs?: number } = {}
  ): Promise<ShiftStatusResponse> {
    const pollMs = options.pollMs ?? 500;
    const deadline = Date.now() + (options.timeoutMs ?? 30_000);
    while (Date.now() < deadline) {
      const status = await this.shiftStatus(hash);
      if (status.status === "finalized" || status.status === "rejected") {
        return status;
      }
      await sleep(pollMs);
    }
    throw new Error(`Timeout waiting for finalization of ${hash}`);
  }

  /** Poll until a quorum forms for the given tick. */
  async waitForQuorum(
    tick: number,
    options: { pollMs?: number; timeoutMs?: number } = {}
  ): Promise<QuorumResponse> {
    const pollMs = options.pollMs ?? 500;
    const deadline = Date.now() + (options.timeoutMs ?? 30_000);
    while (Date.now() < deadline) {
      const res = await this.getQuorum(tick);
      if (res.finalized) return res;
      await sleep(pollMs);
    }
    throw new Error(`Timeout waiting for quorum on tick ${tick}`);
  }

  /** Subscribe to real-time state snapshots via WebSocket. */
  subscribeSnapshots(callback: (snap: StateSnapshot) => void): () => void {
    this.snapshotListeners.push(callback);
    if (!this.ws) this.connectWs();
    return () => {
      this.snapshotListeners = this.snapshotListeners.filter(
        (cb) => cb !== callback
      );
    };
  }

  private connectWs() {
    if (typeof WebSocket === "undefined") {
      // Node environment: try to import isomorphic-ws lazily.
      void import("isomorphic-ws").then((mod) => {
        this.ws = new mod.default(this.wsUrl) as unknown as WebSocket;
        this.bindWs();
      });
      return;
    }
    this.ws = new WebSocket(this.wsUrl);
    this.bindWs();
  }

  private bindWs() {
    if (!this.ws) return;
    this.ws.onmessage = (event) => {
      try {
        const snap = JSON.parse(
          typeof event.data === "string" ? event.data : event.data.toString()
        ) as StateSnapshot;
        if (snap.commutative_applied !== undefined) {
          this.snapshotListeners.forEach((cb) => cb(snap));
        }
      } catch {
        // ignore malformed messages
      }
    };
    this.ws.onclose = () => {
      this.ws = null;
      if (this.snapshotListeners.length > 0) {
        setTimeout(() => this.connectWs(), 1_000);
      }
    };
  }
}
