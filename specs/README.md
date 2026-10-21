# Rewrite design and behavior contract

This project is a fresh Rust implementation of `mgh`. Everything needed to
implement its behavior and architecture is specified here. The current code is
only the Cargo starter; the application behavior is not implemented yet.

- [Architecture proposal](architecture-proposal.md)
- [Architecture proposal SVG](architecture-proposal.svg)
- [Architecture proposal PNG](architecture-proposal.png)
- [Editable D2 source](architecture-proposal.d2)
- [Requirements and scope](requirements.md)
- [CLI and presentation](cli.md)
- [Configuration and permissions](configuration.md)
- [Authentication, hooks, guards and shells](integration.md)
- [Messages, prompts and rendering](messages.md)
- [Acceptance and testing](acceptance.md)

The behavior specifications define required outcomes. The architecture proposal
defines module ownership, simple interfaces and implementation boundaries.
Use these documents to design code and tests within this project. Proposed file
names and private implementation details can evolve as the design is implemented.

Regenerate the proposal diagrams with D2 0.9.0:

```sh
d2 specs/architecture-proposal.d2 specs/architecture-proposal.svg
d2 specs/architecture-proposal.d2 specs/architecture-proposal.png
```
