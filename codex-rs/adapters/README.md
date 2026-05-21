# Adapter Crates

This folder is reserved for outer-layer adapter crates that translate UI,
wire-protocol, or concrete runtime types into the small domain and port crates.

Adapter crates may depend on concrete integration crates. Domain, session,
turn, thread API, tool-domain, context-domain, and runtime-domain crates must
not depend on crates in this folder.
