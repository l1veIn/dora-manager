# dm-message

`dm-message` is a sink-style interaction node.

- It accepts a single `message` input and forwards it into the DM interaction plane.
- String payloads are treated as inline content by default.
- If the payload resolves to an existing file path inside the current run output directory, it is emitted as an artifact-backed message instead.

Use it when a dataflow branch should end in a human-visible message rather than continue through more compute nodes.
