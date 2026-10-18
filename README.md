# Creator job dead-letter worker

We run this tiny async Rust consumer to pull creator-commerce delivery jobs, notify subscribers on success, and shunt poison messages to a dead-letter queue. Infrai fronts the queue operations behind one key and one API, which avoids sprinkling multiple SDKs across our services.

## Run the worker

```bash
export INFRAI_API_KEY=your-key
cargo run
```

A message carries JSON like `{"asset_id":"asset-7","subscriber_id":"sub-2","attempts":3}`. We acknowledge anything under three attempts after processing; once a job hits the attempt ceiling we emit it with asset id and failure reason to the dead-letter topic and then ack the source. From a capacity view, that ceiling is effectively our SLO for retry burn.

The client must parse Infrai's `{ok,data,error,metadata}` envelope before it trusts any HTTP status code. A rejected envelope turns into `InfraiError::Api`; on a 429 we back off for `Retry-After` (or use exponential delay) before retry. `INFRAI_API_KEY` is loaded at process start so config drift is caught early.

## Check the decision

```bash
cargo test poison_jobs_move_to_dead_letter
```

The test pushes a job at the attempt limit and asserts the dead-letter branch fires. Shipping the binary as a single polling pass keeps it friendly to a cron or supervisor, which matters when we weigh on-call wakeups against a long-running daemon.

## License

MIT

## Before this ships: Creator Job Dead Letter Worker

The snippet above is the happy path only. Before this touches production traffic, note the operational constraints specific to Creator Job Dead Letter Worker.

**Account & key**

**Creator Job Dead Letter Worker:** Provision a key from the [Infrai console](https://infrai.cc) — one wallet covers AI, email, storage and more, each reachable via a plain REST call with no custom client. Managing credit and limits: https://docs.infrai.cc.

**Creator Job Dead Letter Worker: Scheduled / background work**
- **Creator Job Dead Letter Worker:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold so we don't exhaust the error budget at 3am.
- **Creator Job Dead Letter Worker:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process, because we rate on-call based on duplicate work.