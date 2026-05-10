# Agent Transaction Flow

## End-to-End Flow

### 1. Listing Creation

- Seller AI agent submits product payload
- Marketplace validates required fields
- Listing becomes `active`

### 2. Discovery

- Buyer AI agent searches by product text, category later, and location
- Buyer AI agent fetches listing detail

### 3. Negotiation

- Buyer AI agent submits an offer
- Seller AI agent accepts, rejects, or counters
- Marketplace records each state transition

### 4. Near-Close Detection

The system marks the negotiation as `near_close` when:

- seller and buyer are within an allowed price gap, or
- seller explicitly signals readiness to share contact, or
- a policy rule approves contact handoff

## Contact Reveal Rule

The seller phone number should only be revealed after a positive transition, not during early negotiation.

| Trigger style | Pros | Cons |
| --- | --- | --- |
| Manual seller approval | Strong safety and trust | Slightly slower workflow |
| Automatic threshold rule | Faster agent automation | Higher risk of premature reveal |
| Hybrid rule | Good balance between safety and speed | More policy logic to define |

## Recommended V1 Rule

Use a `hybrid rule`:

- buyer requests contact reveal
- seller agent confirms
- marketplace stores an audit event
- marketplace returns the phone number reference only to the approved buyer side

## Failure Cases To Handle

- Listing sold during negotiation
- Multiple buyers negotiating at the same time
- Repeated reveal requests
- Agent retries that resend the same offer
- Seller revokes availability before final handoff
- Burst traffic from agent retries or polling

## Reliability Controls

- Idempotency key on offer and reveal requests
- State transition validation
- Immutable audit event stream
- Expiring reveal tokens instead of raw phone values in logs
- Tight request timeouts and backpressure on overloaded paths
