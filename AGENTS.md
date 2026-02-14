# AGENTS.md

## Project Overview

Real-time voice-to-text input on Linux.

- dictyped (Rust): a service that records audio and talks to a dictation backend.
- dictype-fcitx (C++): Fcitx addon to talks dictyped and integrates with Fcitx.

# Repository Layout

- C++
    - `DictypeFcitx/` — Fcitx addon.
- Rust
    - `crates/dictyped/` — `dictyped` binary (service entrypoint, audio/session streams, client wiring).
    - `crates/base-client/` — shared ASR client traits, stream types, gRPC server helpers.
    - `crates/paraformer-v2-client/` — Paraformer V2 backend client implementation.
    - `crates/qwen-v3-client/` — Qwen V3 backend client implementation.
    - `crates/config-tool/` — profile/config persistence helpers used by service/client code.

## Service Architecture Notes (LLM Context)

- High-level flow
    - Audio is captured by `dictyped`.
    - `dictyped` streams audio to an ASR backend via client crates.
    - Final/partial transcripts are returned over gRPC and consumed by the Fcitx addon.
- Protocol boundary
    - Treat `proto/dictype.proto` as the source of truth for cross-language service contracts.
    - When changing protocol fields or RPC behavior, update both Rust and C++.
- Backend abstraction
    - `base-client` defines common interfaces.
    - Backend-specific crates (`paraformer-v2-client`, `qwen-v3-client`) should stay interchangeable behind shared
      traits.

## Coding Style

- Assume the latest and modern versions of everything.
- Structure code in `Vertical Slice Architecture` style.
- Organize code with locality in mind.
- Prefer `code reuse after a small refactoring` to `adding a parallel implementation for short-time convenience`.
- Avoid `default arguments` unless absolutely necessary, don't do it in the name of backward compatibility.
- Only leave comments when a) express high-level intents when they are not obvious; b) when there are hacks worth
  explaining.
- Avoid testing things that are covered by the type system already.
