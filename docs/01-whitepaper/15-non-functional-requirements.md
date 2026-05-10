# Non-Functional Requirements

## Goal

Define the first explicit non-functional targets for:

- latency
- throughput
- availability
- quotas
- error budgets

These targets should guide implementation and measurement, not marketing claims.

## Service Scope

The system includes:

- core server
- MCP server for desktop agents
- Android and iOS clients using the backend

The main hot paths are:

- listing reads
- listing search
- negotiation writes
- contact reveal actions

## Latency Targets

### Recommended V1 Targets

| Path | Target |
| --- | --- |
| `GET /v1/listings/{id}` p95 | `< 100 ms` |
| `GET /v1/listings/search` p95 | `< 200 ms` |
| `POST /v1/listings` p95 | `< 250 ms` |
| `POST /v1/negotiations` p95 | `< 250 ms` |
| `POST /v1/negotiations/{id}/offers` p95 | `< 250 ms` |
| `POST /v1/contact-reveals/{id}/approve` p95 | `< 300 ms` |

### Recommended Tail Targets

| Path Class | p99 Target |
| --- | --- |
| read-heavy endpoints | `< 300 ms` |
| write-heavy endpoints | `< 500 ms` |

## Throughput Targets

The `50k+ RPS` goal should be split by path class.

### Recommended Measurement Shape

| Class | Initial Target |
| --- | --- |
| listing read RPS | very high, primary candidate for `50k+` benchmark |
| search QPS | moderate to high, benchmark separately |
| write RPS | lower than read path, correctness first |
| reveal/finalization RPS | low volume, correctness first |

### Rule

Do not present one blended throughput number unless the request mix is explicitly defined.

## Availability Targets

### Recommended V1

| Target | Recommendation |
| --- | --- |
| monthly availability | `99.9%` |
| planned maintenance | should be minimized and announced |
| degraded mode | read/search may stay up even if some write paths are throttled |

## Quota Targets

### Listing creation quotas

| Seller Trust Level | Suggested Limit |
| --- | --- |
| new seller | `5 listings per day`, `2 listings per hour` |
| established seller | `25 listings per day`, `10 listings per hour` |
| trusted seller | `100 listings per day`, `30 listings per hour` |

These are recommended first numbers, not permanent policy. They should be reviewed after real traffic data exists.
Use the Postgres-backed `phase5_bench` path before changing them so the quota review is based on the real storage path, not the in-memory fallback.

## Tuning Targets

After the Postgres rerun, review these first:

- new seller daily and hourly listing quota
- per-token create-write rate
- per-IP search request rate
- trusted seller lift relative to `new` and `verified`

If the benchmark still shows pressure, adjust the search path and index strategy before widening write quotas.

### Search and negotiation quotas

Recommended first limits:

| Path Class | Suggested First Limit |
| --- | --- |
| search requests | `60 requests/minute` per token, `300 requests/minute` per IP |
| listing create writes | governed by seller trust quota plus `10 requests/minute` per token |
| negotiation open | `30 requests/hour` per buyer agent |
| negotiation offer submit | `120 requests/hour` per negotiation participant |
| contact reveal request | `10 requests/hour` per negotiation participant |
| contact reveal approve | `30 requests/hour` per seller credential |

### Abuse Escalation Thresholds

Recommended first thresholds:

- more than `3` duplicate listing attempts in `10 minutes` -> soft block and review signal
- more than `5` create-listing failures in `15 minutes` -> temporary create cooldown
- more than `20` offer submissions in `5 minutes` for one negotiation -> spam flag
- repeated reveal-request replays with same negotiation -> force idempotency enforcement and alert

## Error Budget Targets

### Recommended V1 Error Budget

| Metric | Target |
| --- | --- |
| 5xx rate monthly | `< 0.1%` of requests |
| invalid transition failures | expected and should be explicit, not counted as platform instability |
| authz failures | expected and should be machine-readable |

### Interpretation Rule

Do not treat client-caused `4xx` errors as service instability, but do monitor them for abuse and UX issues.

## Reliability Requirements

- every write path must support idempotency where replay is dangerous
- every state transition must be version-checked
- reservation conflicts must fail cleanly
- contact reveal must never leak raw phone data through generic read paths

## Observability Requirements

Minimum required signals:

- request count by endpoint
- latency by endpoint
- error count by code
- rate-limit and quota rejections
- reservation conflict count
- reveal approval/rejection counts

## Load Test Requirements

Before claiming throughput numbers, define:

- request mix
- payload sizes
- auth cost
- search-query distribution
- read/write ratio
- p95 and p99 targets

## First Benchmark Profiles

### Profile A: Listing Read Heavy

- `90%` listing reads
- `8%` search
- `2%` writes

### Profile B: Search Heavy

- `60%` search
- `30%` listing reads
- `10%` writes

### Profile C: Negotiation Burst

- `40%` listing/search reads
- `50%` negotiation writes
- `10%` reveal/finalization actions

## Best Next Moves

1. define the first benchmark profiles for read, search, and write traffic
2. attach these targets to the server and API specs
3. create observability checklists before coding hot paths
