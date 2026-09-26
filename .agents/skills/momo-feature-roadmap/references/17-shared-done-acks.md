# 17 shared-done-acks

Source: upstream discussion #4420 "Optional shared Done acknowledgements across my devices"
(follows upstream issue #4160 and discussions #651, #691).

## Problem

Each client tracks which completions it has displayed. One person using a desktop and a phone reviews
a finished agent on the phone, but the desktop keeps its Done badge. Over a day the badges stop
meaning "still needs my review".

## Behavior

```toml
[ui]
done_acknowledgement = "per_client"   # default, today's behavior
# done_acknowledgement = "shared"     # viewing a completion on any client clears it everywhere
```

- `shared`: when any client displays a specific completion, the server records it as acknowledged
  and every client in shared mode clears that Done badge. A later completion shows Done again.
- Clients in `per_client` mode keep independent review queues even when others use `shared`.
- CLI/API status semantics (`idle` vs `done`) stay as documented.

## Classification

The acknowledgement is a shared runtime fact (server state + event). Whether a client follows it is a
client presentation preference.

## Stop-and-ask checkpoint

1. How completions are identified today (per-completion id vs status transitions) and where
   per-client "displayed" state lives (`src/client/shell/endpoint_agent_state.rs`,
   `src/client/shell/endpoints.rs`, seen-state in `src/app/mod.rs`).
2. Transport: a new API method (`agent.acknowledge_done { pane_id, completion_id }`) plus an
   optional snapshot/event field. Confirm JSON-optional compatibility and the private-protocol framing.
3. Persistence across server restart: recommended no.

## Tests

- Two simulated clients in shared mode: ack on one clears the other; a new completion re-badges.
- Mixed modes: per-client client unaffected by a shared client's ack.
- Old server without the method: shared mode degrades to per-client with a one-time notice.
