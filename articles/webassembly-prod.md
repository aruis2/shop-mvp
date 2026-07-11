---
title: "WebAssembly în producție — server, edge și înlocuitor pentru containere"
slug: "wasm-prod"
summary: "WebAssembly nu mai e doar pentru browser. Wasmtime, Fermyon și WasmEdge rulează Wasm pe server, edge și IoT. Startup de 100 µs, izolare reală, sigur, portabil — un nou model de computing."
category_path: ["Sisteme și arhitectură", "Virtualizare", "WebAssembly"]
tags: ["WebAssembly", "Wasm", "Wasmtime", "Fermyon", "serverless", "edge", "container", "portabilitate"]
difficulty: "advanced"
reading_time: 20
related: ["edge-ai-serverless", "post-posix", "sisteme-de-operare-concepte", "big-o", "paxos"]
---

## Wasm dincolo de browser

WebAssembly a început ca un target pentru compilare în browser. Azi, Wasm rulează pe **server, edge, embedded**, concurând cu containerele Docker.

## Avantaje față de containere

| Caracteristică | Docker | Wasm |
|---|---|---|
| **Startup** | 1-5 secunde | **50-200 µs** |
| **Dimensiune imagine** | 10-500 MB | **1-10 KB** |
| **Izolare** | OS-level | **Sandbox hardware** |
| **Limbaje** | Orice | Rust, C, Go, JS... |
| **Siguranță** | User namespace | **Verificat înainte de execuție** |

## Ecosistem

- **Wasmtime** — runtime Wasm de la Bytecode Alliance
- **Fermyon Spin** — framework pentru microservicii Wasm
- **WasmEdge** — runtime cu suport AI
- **WASI** — interfața de sistem pentru Wasm

Wasm în producție e încă la început, dar pentru serverless și edge computing, e deja o alternativă viabilă la containere.
