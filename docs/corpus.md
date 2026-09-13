# Portable behavior corpus

`newtui-corpus` addresses [issue #13](https://github.com/Gilamonster-Foundation/newtui/issues/13):
hold another implementation to observable behavior, including the order of keys,
close decisions, and emitted intents. A set of reachable views cannot do this.
The exporter is a non-default workspace member. The core dependency closure and
the existing Explorer implementation are unchanged.

## Existing content-addressing authority

Before adding the carrier, the Newt inventory identified
`newt-core/src/settings_receipt.rs`'s `{id, record}` carrier and
`ContentAddressable` implementation, `content_spill.rs`'s canonical-byte staging,
and the workspace's `content-addressable = "0.1.2"` dependency. The corpus follows
that existing division: JSON transports a record; canonical DAG-CBOR identifies
its content. It introduces no store, append log, raw-edit journal, or Merkle
protocol. `RawContentId`, `MerkleNode`, and `NodeStore` are unnecessary here.

`Corpus::canonical_form` delegates directly to
`content_addressable::canonical::to_canonical_dagcbor`. `ContentId` supplies the
CID, and `ensure_content_id` checks it. Checked canonical ingress uses the crate's
`from_canonical_form`, including its typed canonical round-trip check. There is
no local CBOR encoder or hash implementation.

The Python consumer uses the published 0.1.2 face of the same library. Go uses
[go-ipld-prime](https://github.com/ipld/go-ipld-prime)'s DAG-CBOR codec and
[go-cid](https://github.com/ipfs/go-cid) with the same CIDv1/DAG-CBOR/BLAKE3-256
profile. The official [Go JSON implementation](https://github.com/go-json-experiment/json)
validates Unicode scalars and duplicate fields before a decoder can replace
malformed text. It also marshals native observables strictly, so invalid UTF-8
cannot compare equal to a valid U+FFFD. These modules are pinned in `go.mod` and
`go.sum`; they do not enter any Rust build.

## Schema v1

The carrier is `{ "id": "<canonical base32-lower CID>", "corpus": {...} }`.
The ID addresses only the `corpus` record, whose schema is
`newtui.behavior-corpus/v1`. JSON whitespace and map order do not affect it.

| Field | Meaning |
| --- | --- |
| `component` | Host-owned, versioned behavior contract, e.g. `newtui.example-dial/v1` |
| `seed` | Contract-defined fresh inputs; portable JSON values |
| `domain` | Exact key alphabet, maximum states, maximum depth |
| `exploration` | States, transitions, terminal states, exhaustion, verdict, incomplete reason, replay issues |
| `properties` | Ordered names, observation/applicable/held counts, and final outcome |
| `events` | Ordered state and transition observations, each with a full replay trace and positional property checks |

A trace contains its actual fresh `initial` snapshot and an ordered `steps`
array. Every step contains `key`, `flow`, and an `after` snapshot. Each snapshot
contains the complete observable `View` and the host's projected `intent`.
A view includes title, footer, and all row labels, values, notes, selected flags,
and adjustable flags. A flow is `stay` or `close` with an `applied` boolean.
Keys use the portable core vocabulary, including one-scalar `char` and `ctrl`
payloads. No terminal-library key codes are persisted.

Property outcomes are `not_applicable`, `held`, or `violated` with a detail.
Checks identify properties by their zero-based position, preserving duplicate
names and first-violation retirement. A missing check is not a successful check.
Replay issues contain key paths and a reason, never fingerprints or their
debug text. Paths are structural observations, not newly minted state IDs.

The seed and intent domain is null, boolean, signed 64-bit integer, UTF-8 string,
array, or map with unique string keys. Floats, unsigned values above i64::MAX,
duplicate fields, unknown schema fields, and invalid Unicode are rejected.
`{"/":"value"}` remains an ordinary map in every language. Nesting is limited
to 32 levels independently for each seed or intent; map keys do not add a level.

Bounds are at most 64 alphabet keys, depth 64, 50,000 states, 64 properties,
and 100,000 observations. An empty alphabet or zero search bound can describe
incomplete evidence; it cannot manufacture conformance. JSON ingress/emission
and checked canonical ingress have an 8 MiB limit. These are serialized limits,
**not an in-memory recorder budget**: complete replay traces are collected and
encoded before output limits are checked. Choose a suitably small finite domain.

## Capture and acceptance

`capture` wraps `Component` and each `Property`, delegates to the existing
`Explorer`, and records only observations that the explorer actually judges.
The recorder's extra property always returns NotApplicable and is removed from
the returned report by its known position. It does not replace a user property,
alter applicability, or supply a passing verdict. Replay-only key applications
do not become extra explored edges. A close encountered during reconstruction
cannot fabricate later transitions.

The factory and intent projection must be pure. Every replay must start from
the actual fresh view and intent captured from the seed factory. The host must
describe those same inputs in `seed` and include every state distinction that
can affect future behavior in its internal fingerprint. The corpus neither
serializes nor attempts to infer that private state representation.

`export` validates the typed record before minting it. Ingress checks identity,
schema and value domains, discovered departures, exact replay prefixes, terminal
observations, property positions and retirement, counts, and bound consistency.
No transition may occur after the declared state cap. An exhausted stable walk
must include every declared key from every discovered open state.

`check` replays every observation from a fresh consumer and compares its initial
snapshot and every intermediate flow, view, and intent. It then runs the declared
properties against the actual observation. The generic Rust caller must first
select an implementation for the `component` contract ID; the example CLIs do
that explicitly. Native input values and returned view/flow snapshots are detached
from mutable consumer storage. Invalid domains and mismatched behavior fail;
matching incomplete or violated evidence also fails. A CID establishes content
identity, not that a publisher told the truth about its implementation.

## Reproduction and scope

```sh
cargo run -p newtui-corpus -- export-dial > /tmp/dial.json
cargo run -p newtui-corpus -- canonical /tmp/dial.json > /tmp/dial.cbor
cargo run -p newtui-corpus -- check /tmp/dial.json
python newtui-corpus/python/consumer.py /tmp/dial.json
(cd newtui-corpus/go && go run . /tmp/dial.json)
```

The dial contract seeds `level=0, maximum=3`, uses the six navigation/close keys,
and declares 32 states and depth 8. It exhausts four open states, 24 transitions,
eight terminal observations, and 36 observations total. Its canonical payload
is 22,075 bytes, with CID
`bafyr4ifzpp5ebpsr5scyoklyuwkcasqqmlscfpggqhioylqmxcaornrkau`.
All three implementations read the same JSON carrier. Rust also pins its exact
regenerated JSON and canonical bytes; Python and Go independently verify those
bytes and drive their own implementations. Wrong close flows and emitted
intents fail even when their view sets match.

The generic Rust suite additionally exports and replays a real SettingsPanel
with choice, bounded-number, fixed, model and backend rows. This proves the
adapter can carry the existing component without adding a settings schema to
the core. The native example consumers implement only the dial contract.
Crush adoption and linked-pane corpus fixtures are later consumer work.

Install `content-addressable==0.1.2` and `coverage>=7,<8` into the selected Python
environment; use Go 1.26+ for the optional Go consumer. Run
`just corpus-check corpus-coverage corpus-consumers`. The full `just check`,
normal push hook, and CI name this optional package explicitly, enforce the
80% source coverage floors, and run the real negative consumers. Rust's MSRV
gate remains 1.88.
