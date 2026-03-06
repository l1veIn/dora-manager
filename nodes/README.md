# Builtin Nodes

This directory contains builtin DM-managed nodes that ship with the repository.

Discovery order in `dm-core` is:

1. `~/.dm/nodes`
2. repository builtin nodes in the workspace `nodes/` directory
3. extra directories from `DM_NODE_DIRS`

Managed nodes in `~/.dm/nodes` override builtin nodes with the same ID.
