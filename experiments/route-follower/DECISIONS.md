# Owner decisions (2026-10-24, before the experiment started)

1. **Machine sharing: small alongside.** Prototypes built now. Measuring runs engine-only on at most
   2–3 threads while the smart-planner build runs (its sf-plan budgets are wall-clock, so contention
   skews its numbers). Oracle (vAmiga) runs only when `pgrep -fl 'sf-bench|sf-plan'` shows no benchmark.
2. **Success = two columns.** "Reached the exit" and "reached the exit without losing a life", hearts
   lost reported beside them.
3. **Same route = same places, in order.** A follower must pass the route's guide points (cell + coins
   left) in order; slower ways between them are allowed. A run that reaches the exit off the guide
   path is reported separately, not counted as following.
4. **(later, same day) Oracle runs need not wait for idle.** Owner: "ignore it and continue". Still one
   oracle at a time, low priority (`nice`), small thread counts.
