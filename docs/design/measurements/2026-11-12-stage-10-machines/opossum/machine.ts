// Hand-written stand-in for the module `accord build` would write from a
// `machine Breaker` declaration: opossum 10.0.0's state flags and decisions,
// ported faithfully. Only integers, enums and Bools; one function per event,
// and observers. The state crosses the boundary as a plain object of its
// fields; this stand-in checks their types, not the invariants.

export type Phase = "closed" | "open" | "half_open" | "shutdown";

export type Breaker = {
  readonly phase: Phase;
  readonly pendingClose: boolean;
  readonly enabled: boolean;
  readonly warmingUp: boolean;
};

const PHASES: readonly Phase[] = ["closed", "open", "half_open", "shutdown"];

function checkBool(name: string, value: unknown): void {
  if (typeof value !== "boolean") throw new TypeError(`${name} must be a boolean`);
}

function checkBreaker(b: unknown): Breaker {
  if (typeof b !== "object" || b === null) throw new TypeError("breaker must be a Breaker");
  const s = b as Breaker;
  if (!PHASES.includes(s.phase)) throw new TypeError("breaker.phase must be a Phase");
  checkBool("breaker.pendingClose", s.pendingClose);
  checkBool("breaker.enabled", s.enabled);
  checkBool("breaker.warmingUp", s.warmingUp);
  return s;
}

function checkI64(name: string, value: unknown): bigint {
  if (typeof value !== "number") throw new TypeError(`${name} must be a number`);
  if (!Number.isSafeInteger(value)) throw new RangeError(`${name} must be a safe integer`);
  return BigInt(value);
}

// Initial states.

export function initial(enabled: boolean, warmingUp: boolean): Breaker {
  checkBool("enabled", enabled);
  checkBool("warmingUp", warmingUp);
  return { phase: "closed", pendingClose: false, enabled, warmingUp };
}

// A breaker restored from `options.state`, as opossum's constructor does,
// including combinations its own `toJSON` never writes. `openExpired` is the
// shell's comparison of `lastTimerAt` with `resetTimeout`.
export function restored(
  enabled: boolean, warmingUp: boolean, closed: boolean, halfOpen: boolean,
  shutdown: boolean, openExpired: boolean,
): Breaker {
  for (const [n, v] of Object.entries({ enabled, warmingUp, closed, halfOpen, shutdown, openExpired })) checkBool(n, v);
  const base: Breaker = { phase: "closed", pendingClose: halfOpen, enabled, warmingUp };
  if (shutdown) return { ...base, phase: "shutdown", enabled: false };
  if (closed) return base;
  if (!halfOpen) return openExpired
    ? { ...base, phase: "half_open", pendingClose: true }
    : { ...base, phase: "open", pendingClose: false };
  return { ...base, phase: "half_open" };
}

// Observers.

export function isClosed(b: Breaker): boolean { return checkBreaker(b).phase === "closed"; }
export function isOpen(b: Breaker): boolean { return checkBreaker(b).phase === "open"; }
export function isHalfOpen(b: Breaker): boolean { return checkBreaker(b).phase === "half_open"; }
export function isShutdown(b: Breaker): boolean { return checkBreaker(b).phase === "shutdown"; }
export function isPendingClose(b: Breaker): boolean { return checkBreaker(b).pendingClose; }
export function isEnabled(b: Breaker): boolean { return checkBreaker(b).enabled; }
export function isWarmingUp(b: Breaker): boolean { return checkBreaker(b).warmingUp; }

// A protected call runs only when the breaker is closed or holds its
// half-open probe.
export function admits(b: Breaker): boolean {
  const s = checkBreaker(b);
  return s.phase === "closed" || s.pendingClose;
}

// Whether a failure opens the breaker: opossum's `fail()`, with the error
// rate compared exactly, `failures * 100 > threshold * fires`, instead of
// `failures / fires * 100 > threshold` in floating point.
export function opensOnFailure(
  b: Breaker, fires: number, failures: number, volumeThreshold: number, threshold: number,
): boolean {
  const s = checkBreaker(b);
  const f = checkI64("fires", fires), x = checkI64("failures", failures);
  const v = checkI64("volumeThreshold", volumeThreshold), t = checkI64("threshold", threshold);
  if (f < 0n || x < 0n) throw new RangeError("requires fires >= 0 and failures >= 0");
  if (s.warmingUp) return false;
  const halfOpen = s.phase === "half_open";
  if (f < v && !halfOpen) return false;
  return x * 100n > t * f || halfOpen;
}

// Events.

export function close(b: Breaker): Breaker {
  const s = checkBreaker(b);
  return s.phase === "closed" ? s : { ...s, phase: "closed", pendingClose: false };
}

export function open(b: Breaker): Breaker {
  const s = checkBreaker(b);
  return s.phase === "open" ? s : { ...s, phase: "open", pendingClose: false };
}

export function resetElapsed(b: Breaker): Breaker {
  return { ...checkBreaker(b), phase: "half_open", pendingClose: true };
}

export function callAdmitted(b: Breaker): Breaker {
  return { ...checkBreaker(b), pendingClose: false };
}

export function shutdown(b: Breaker): Breaker {
  return { ...checkBreaker(b), phase: "shutdown", enabled: false };
}

export function enable(b: Breaker): Breaker { return { ...checkBreaker(b), enabled: true }; }
export function disable(b: Breaker): Breaker { return { ...checkBreaker(b), enabled: false }; }
export function warmUpEnded(b: Breaker): Breaker { return { ...checkBreaker(b), warmingUp: false }; }
