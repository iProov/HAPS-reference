# HAPS conformance adapter protocol v1

This is an experimental adapter protocol for a draft specification and partial reference
implementation. It is not production-ready. The historical `conformance` name identifies the kit;
passing its finite tests does not establish full protocol conformance or that a person saw and
approved the correct action.

The protocol lets one deterministic runner test implementations in any language. An adapter reads one
JSON object per line from standard input and writes exactly one JSON object per line to standard output.
Diagnostics go to standard error. UTF-8 is required throughout.

The conformance runner currently starts a fresh adapter process for each request, sends one request, and
closes standard input. An adapter may nevertheless serve multiple request lines so it also works with
future persistent runners.

## Envelope

Every request and response contains:

```json
{"protocol":"haps-conformance-adapter-v1","id":"opaque-request-id"}
```

The response repeats the request `id`. Unknown protocols, missing fields, or malformed envelopes return
`status: "error"` with `error.code: "PROTOCOL_ERROR"`.

## Capabilities

Request:

```json
{"protocol":"haps-conformance-adapter-v1","id":"capabilities","operation":"capabilities"}
```

Response:

```json
{
  "protocol":"haps-conformance-adapter-v1",
  "id":"capabilities",
  "status":"ok",
  "implementation":{"name":"example","version":"1.0.0"},
  "haps_versions":["0.4"],
  "operations":["canonicalize_and_hash"]
}
```

## Canonicalize and hash

`document` is the exact candidate HAPS JSON document transported as a JSON string.

```json
{
  "protocol":"haps-conformance-adapter-v1",
  "id":"case-1",
  "operation":"canonicalize_and_hash",
  "document":"{\"amount\":\"250.00\"}"
}
```

Success returns canonical JSON as a string and the HAPS-prefixed digest:

```json
{
  "protocol":"haps-conformance-adapter-v1",
  "id":"case-1",
  "status":"ok",
  "canonical":"{\"amount\":\"250.00\"}",
  "hash":"sha256:w53HIJPJgMdAASIifhbQwH88m3wnzopxMEBiCuvqi0w"
}
```

The runner independently hashes the returned canonical UTF-8 bytes and requires that digest to equal
both `hash` and the pinned expected hash. It also submits `canonical` again and requires a fixed point.

Rejection returns a stable error code:

```json
{
  "protocol":"haps-conformance-adapter-v1",
  "id":"case-1",
  "status":"error",
  "error":{"code":"DUPLICATE_KEY"}
}
```

The adapter contract defines the following codes. The current suite directly exercises only
`NON_INTEGER_NUMBER`, `INTEGER_OUT_OF_RANGE`, `DUPLICATE_KEY`, `SYNTAX`, and `TRAILING_CONTENT`:

| Code | Meaning |
| --- | --- |
| `NON_INTEGER_NUMBER` | A JSON number contains a fractional part or exponent. |
| `INTEGER_OUT_OF_RANGE` | An integer is outside ±(2^53 − 1). |
| `DUPLICATE_KEY` | An object repeats a member name. Detection must precede ordinary last-wins parsing. |
| `SYNTAX` | The JSON document is otherwise malformed. |
| `TRAILING_CONTENT` | Content follows one complete JSON value. |
| `DEPTH_EXCEEDED` | The implementation's documented nesting bound was exceeded. |
| `UNSUPPORTED_OPERATION` | The request operation is not implemented. |
| `PROTOCOL_ERROR` | The outer adapter envelope is invalid. |
| `INTERNAL_ERROR` | The adapter failed without classifying the input. This never satisfies a vector expecting a specific rejection. |

## Scope boundary

Passing this protocol's v0.4 suite records agreement with the pinned canonicalization/hash vectors
and listed malformed-input examples only. It does not prove the associated properties for arbitrary
input or test every requirement in this adapter contract. It is not evidence of HAPS-PP-0.4,
HAPS-RP-0.4, any PoHP level, signature or issuer verification, replay protection, provider certification,
presence-signal quality, correct display, or human approval. Intended security and approval outcomes
depend on implementation and deployment conditions beyond this adapter.

The runner uses fresh local subprocesses with the invoking user's permissions, without sandboxing.
It checks output size after collection and applies a timeout per request; resource isolation and
review of adapters are the caller's responsibility.
