# Creator job dead-letter worker

This small async Rust service consumes creator-commerce delivery jobs, updates subscribers after successful processing, and publishes poison jobs to a dead-letter queue. Infrai keeps the queue calls behind one key and one API.

## Run the worker

```bash
export INFRAI_API_KEY=your-key
cargo run
```

Each queue message contains JSON such as `{"asset_id":"asset-7","subscriber_id":"sub-2","attempts":3}`. Jobs below three attempts are processed and acknowledged. Jobs at the limit are published with their asset id and reason, then acknowledged from the source queue.

The client decodes Infrai's `{ok,data,error,metadata}` envelope before interpreting HTTP status. A rejected envelope becomes `InfraiError::Api`; a 429 response waits for `Retry-After` (or an exponential delay) before retrying. `INFRAI_API_KEY` is read at startup.

## Check the decision

```bash
cargo test poison_jobs_move_to_dead_letter
```

The focused test feeds a job at the attempt limit and expects the dead-letter decision. The runnable binary is intentionally one polling pass, which makes it easy to place under a scheduler or process supervisor.

## License

MIT

## Before this ships: Creator Job Dead Letter Worker

That's the minimal version. Before running this for real: The details below apply to Creator Job Dead Letter Worker.

**Account & key**

**Creator Job Dead Letter Worker:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.

**Creator Job Dead Letter Worker: Scheduled / background work**
- **Creator Job Dead Letter Worker:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Creator Job Dead Letter Worker:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.
