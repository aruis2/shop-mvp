---
title: "Cross-compilare — Arhitectură și practică"
slug: cross-compilare
summary: "Ce este cross-compilarea, cum funcționează lanțul de compilare și de ce LLVM IR nu e chiar universal. Incluzând cazul practic x86_64 → ARM64."
difficulty: "avansat"
tags: ["cross-compilare", "LLVM", "ARM", "toolchain", "rust"]
category_path: ["Sisteme și arhitectură"]
reading_time_minutes: 12
related_concepts: ["LLVM IR", "BIOS", "sccache", "mold"]
---

# Cross-compilare — Arhitectură și practică

## Introducere

Cross-compilarea este procesul de a compila cod pe o arhitectură (gazdă) pentru a rula pe o altă arhitectură (țintă). De exemplu, un desktop x86_64 compilează un binar pentru un telefon ARM64. Fără cross-compilare, ai compila direct pe dispozitivul țintă — care e adesea mai lent.

## De ce nu e trivială?

Cross-compilarea pare ușoară în teorie — "e doar un compilator, nu?" — dar în practică implică:

1. **Toolchain diferit** — compilatorul, linker-ul, asamblorul trebuie să fie pentru țintă
2. **Biblioteci C** — `libc`, `libm`, `libpthread` etc. trebuie să existe pentru ținta respectivă
3. **Sysroot** — directorul cu headere și biblioteci ale sistemului țintă
4. **Linker** — produce binarul final; trebuie să știe formatul țintei (ELF, PE, Mach-O)
5. **Runtime differences** — aliniere TLS, paginație, convenții de apel

## Lanțul de compilare Rust

Canonical:
```
Cod Rust (.rs)
    │
    ▼
rustc (frontend) → HIR → MIR → LLVM IR
    │                                    │
    │                         (optimizări LLVM)
    │                                    │
    ▼                                    ▼
Librării (crate) + std        LLVM Backend (codegen)
    │                                    │
    └──────────────┬─────────────────────┘
                   ▼
            Linker (ld / lld / mold)
                   │
                   ▼
            Binar executabil
```

### LLVM IR nu e universal

O concepție greșită comună: "LLVM IR e cross-platform, de ce nu compilezi la IR pe x86 și codegen pe ARM?"

Răspunsul: **LLVM IR conține informații specifice țintei** încă din faza de generare:

```llvm
; LLVM IR pentru x86_64
target triple = "x86_64-unknown-linux-gnu"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-..."
; ⚠️ pointer = 64 biți, dar deja "colorat" pentru x86

; LLVM IR pentru ARM64
target triple = "aarch64-unknown-linux-gnu"
target datalayout = "e-m:e-i8:8:32-i16:16:32-i64:64-..."
; ⚠️ alt alignment, altă convenție
```

Nu poți lua LLVM IR de pe x86_64 și să-l bagi în `llc` pe ARM64 — va crăpa. Nici măcar aceleași opt-pass-uri nu se aplică la fel.

## Ce e un toolchain?

Un toolchain cross = compilator + linker + biblioteci pentru o altă arhitectură:

```
Toolchain pentru aarch64-unknown-linux-gnu:
├── aarch64-linux-gnu-gcc      ← compilator C
├── aarch64-linux-gnu-ld       ← linker (GNU ld)
├── aarch64-linux-gnu-as       ← asamblor
├── aarch64-linux-gnu-objdump  ← disasamblor
├── aarch64-linux-gnu-strip    ← eliminare simboluri
└── sysroot/
    └── usr/
        ├── include/           ← headere C (stdio.h, etc.)
        └── lib/               ← libc.so, libm.so, etc.
```

Rust include std pentru ținta respectivă prin `rustup target add`:
```bash
rustup target add aarch64-unknown-linux-gnu
# → descarcă rust-std pentru ARM64 (~27 MB)
```

Dar std-ul Rust e doar o parte. Dependințele C (openssl-sys, libpq, etc.) au nevoie de toolchain-ul C complet.

## Bionic vs glibc — S22 (Android) vs Linux

Adevărata capcană:

| Sistem | libc | Linker | Aliniere TLS |
|--------|------|--------|:-----------:|
| Ubuntu (x86_64) | glibc | ld-linux-x86-64.so.2 | 8 |
| Ubuntu (ARM64) | glibc | ld-linux-aarch64.so.1 | 8 |
| **Android Bionic** | **Bionic** | **linker64** | **64** |

Când cross-compilăm `aarch64-unknown-linux-gnu` și încercăm să rulăm pe S22 (Android/Termux), primim:

```
error: executable's TLS segment is underaligned:
       alignment is 8 (skew 0), needs to be at least 64 for ARM64 Bionic
```

**Bionic** (Android) e o implementare diferită de libc — mai mică, optimizată pentru dispozitive mobile. Nu e compatibilă binar cu glibc, deși ambele sunt "Linux ARM64".

De aceea, cross-compilarea pentru Android necesită **Android NDK**, nu toolchain-ul Linux generic.

## Cazul practic: x86_64 → ARM64 (S22)

Configurația noastră:

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]   # linker rapid

[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"                # cross-linker

[env]
RUSTC_WRAPPER = "sccache"                        # cache distribuit
```

### Rezultate

| Operație | Desktop nativ | Desktop cross | S22 direct |
|----------|:-----------:|:------------:|:---------:|
| cargo check (incremental) | **0.05s** 🏆 | **0.57s** | **~39s** |
| cargo build (primul) | ~3min | ~3min | ~2min |
| cargo build (incremental) | **0.05s** 🏆 | ~0.6s | **nu** (alt target) |

### Workflow hibrid

Cross-compilarea e excelentă pentru **verificare rapidă** (`cargo check --target aarch64...` în 0.57s), dar pentru **binar executabil pe Android**, compilăm direct pe S22:

```bash
# Verificare rapidă pe desktop (0.57s)
cargo check --target aarch64-unknown-linux-gnu

# Build final pe S22 (~2min)
bash scripts/build-remote.sh build
```

## Cross-compilare pentru Android (avansat)

Dacă vrei cross-compilare completă desktop → Android:

```bash
# 1. Pe S22: instalezi NDK
pkg install ndk-multilib        # ~200MB

# 2. Copiezi toolchain-ul pe desktop
scp -P 8022 u0_a481@S22_IP:/data/data/com.termux/files/usr/bin/aarch64-linux-android-clang* .
scp -rP 8022 u0_a481@S22_IP:/data/data/com.termux/files/usr/aarch64-linux-android .

# 3. Configurezi cargo
# .cargo/config.toml
[target.aarch64-linux-android]
linker = "/path/to/aarch64-linux-android-clang"

# 4. Acum cross-compilezi direct
cargo build --target aarch64-linux-android   # ~0.05s pe desktop!
```

## Concluzie

Cross-compilarea e un instrument puternic, dar nu e magic. LLVM IR nu e universal, toolchain-ul C e necesar pentru dependințe, iar fiecare platformă (Linux, Android, iOS) are particularitățile ei. În practică, un **workflow hibrid** — verificare cross pe desktop, build final pe țintă — e cel mai eficient raport efort/rezultat.
