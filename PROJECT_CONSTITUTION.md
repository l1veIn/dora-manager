# Dora Manager Project Constitution

## 1. Vision

Dora Manager exists to help make application development feel more like assembling Lego bricks than rebuilding the same shell from scratch.

Its long-term direction is not only to manage dora-rs, but to prove a broader construction model:

- recurring display layers can become reusable nodes
- recurring configuration layers can become reusable nodes
- recurring interaction layers can become reusable nodes
- recurring output and feedback layers can become reusable nodes
- truly unique product logic can stay small and focused

If this works, many future tools and applications will not need to reimplement an entire UI, control plane, and feedback loop for every new project. They can compose those capabilities on top of a shared runtime driven by nodes and dataflows.

## 2. Mission

Dora Manager turns dora-rs from a powerful low-level orchestration engine into a usable application-building and management layer.

It must help developers:

- discover and manage nodes
- define and run dataflows naturally
- observe runtime state clearly
- interact with running systems directly
- reuse common app capabilities instead of rebuilding them

## 3. Current Product Bet

The current bet is:

1. dora-rs is a strong timing opportunity because it provides high-performance, zero-copy, multi-language orchestration primitives.
2. A large amount of application work is duplicated across projects.
3. The duplicated layers should gradually be lifted into reusable node capabilities.
4. Dora Manager can become the management, composition, and interaction layer that makes this practical.

## 4. Current Stage

This stage is about proving the path, not maximizing breadth.

Current priorities:

1. first-time startup must be understandable
2. first successful run must be achievable quickly
3. edit, run, inspect, iterate must feel like one coherent loop
4. runtime state, failure, stop, and recovery must be trustworthy
5. reusable display, config, interaction, and feedback capabilities must begin to solidify as node patterns

## 5. Target Users

Current priority users:

1. solo developers
2. early technical adopters trying dora-rs
3. advanced builders validating nodes, flows, and interaction loops
4. engineers exploring a reusable application-construction model

Not the current priority:

- large-team governance
- enterprise permission systems
- distributed cluster scheduling
- broad platform abstraction for its own sake

## 6. Decision Order

When tradeoffs conflict, decide in this order:

1. alignment with the long-term vision
2. first-success rate
3. clarity and predictability
4. runtime truthfulness and stability
5. debugging and feedback quality
6. reduction of repeated wheel-building
7. implementation cost
8. architectural elegance
9. feature breadth

## 7. Product Principles

### 7.1 Reuse beats repetition

If a capability keeps reappearing across products, prefer turning it into a reusable node, contract, or runtime feature instead of reimplementing it per project.

### 7.2 First impression matters

The first startup path, first page, first click, first flow, and first failure all shape trust. Treat them as core product surfaces.

### 7.3 Do not make users guess

At important moments, the product should help the user understand:

- what is happening
- what just happened
- what failed
- what to do next

### 7.4 Fix truth before polish

If frontend messaging and backend reality disagree, fix the system semantics first.

### 7.5 Demos must build confidence

Demo flows, starter paths, shortcuts, and sample nodes must help establish trust, not generate noise, stale links, or fragile illusions.

### 7.6 Main path before edge path

Before expanding breadth, keep improving the core loop:

startup -> run -> understand -> modify -> rerun

### 7.7 Core stays node-agnostic

`dm-core` should not become a pile of special cases for individual nodes. Common behavior belongs in shared contracts and shared runtime logic.

## 8. What Does Not Count As Progress

The following do not count as real progress unless they also improve the core product loop or the reusable-node vision:

- adding a feature with no adoption signal
- making the UI prettier without making it clearer
- making code cleaner without changing product capability
- adding an abstraction layer with no user benefit
- optimizing low-frequency edge cases before the main path works

## 9. Agent Operating Rules

Agents working on Dora Manager should default to:

1. prioritize first-success blockers
2. validate real behavior, not only code assumptions
3. fix backend truth before frontend spin
4. work in narrow rounds with explicit goals
5. keep a round log for long-running efforts
6. use realistic user simulation to break ties when direction is unclear
7. prefer reusable capability extraction when repeated product needs appear

## 10. Amendment Rule

This constitution is durable, but not frozen.

It may be revised when one of the following signals appears:

- repeated dogfooding shows that the current priorities are wrong
- the actual target user proves to be different from the assumed target user
- a stronger product direction emerges from real use, not only theory
- the current wording causes bad steering decisions across multiple rounds

Amendments should be deliberate:

1. identify the triggering signal
2. state what existing clause is no longer serving the project
3. write the proposed replacement
4. explain what decisions will change because of it

## 11. North Star Adjustment Rule

The north star can be refined, but not casually rewritten.

Adjust it only when:

- the market or technical timing materially changes
- repeated real-user evidence contradicts the existing north star
- the project discovers a more truthful expression of the same underlying mission

Do not adjust the north star just to justify local convenience, short-term frustration, or a single implementation preference.

## 12. Final Test

Dora Manager is succeeding if it increasingly proves this statement:

Developers should not have to rebuild the same display, configuration, interaction, output, and feedback layers every time. More of that work should become reusable, composable, runtime-driven building blocks on top of dora-rs.
