// Does a call admitted while the breaker is closed, and resolving after the
// reset timeout, close the half-open breaker without a probe?
// Usage: node stale-success.cjs <opossum folder>
const CircuitBreaker = require(require('path').resolve(process.argv[2]));
const action = (ms, ok) => new Promise((resolve, reject) =>
  setTimeout(() => (ok ? resolve('ok') : reject(new Error('boom'))), ms));
const breaker = new CircuitBreaker(action,
  { timeout: false, resetTimeout: 100, errorThresholdPercentage: 1 });
const log = [];
for (const e of ['open', 'halfOpen', 'close', 'reject']) breaker.on(e, () => log.push(e));
breaker.fire(400, true).then(() => log.push('slow-success'), () => {}); // admitted while closed
breaker.fire(1, false).catch(() => {}); // opens the breaker
setTimeout(() => log.push(`t=250 halfOpen=${breaker.halfOpen} pendingClose=${breaker.pendingClose}`), 250);
setTimeout(() => {
  console.log(log.join(' | '));
  breaker.shutdown();
}, 600);
