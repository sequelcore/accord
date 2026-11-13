// Usage: node invariants.cjs <opossum folder>
// Do the invariants first named in accord#82 hold of opossum 10.0.0 through its public API?
const CircuitBreaker = require(require('path').resolve(process.argv[2]));
const ok = () => Promise.resolve('ran');
const show = (name, b) => console.log(name.padEnd(34), JSON.stringify({ closed: b.closed, open: b.opened, halfOpen: b.halfOpen, pendingClose: b.pendingClose, shutdown: b.isShutdown, enabled: b.enabled }));
(async () => {
  // shutdown is permanent?
  const a = new CircuitBreaker(ok); a.shutdown(); a.close(); show('shutdown, then close()', a);
  console.log('  fire after that:', await a.fire().then(v => v, e => e.code));
  // open admits no call?
  const b = new CircuitBreaker(ok); b.open(); b.disable(); show('open, then disable()', b);
  console.log('  fire while open:', await b.fire().then(v => v, e => e.code));
  // pending close implies half-open?
  const c = new CircuitBreaker(ok, { state: { closed: true, halfOpen: true } }); show('restored {closed, halfOpen}', c);
  const d = new CircuitBreaker(ok, { state: { shutdown: true, halfOpen: true } }); show('restored {shutdown, halfOpen}', d);
  for (const x of [a, b, c, d]) x.shutdown();
})();
