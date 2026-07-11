---
title: "mold + sccache — Accelerare extremă a compilării Rust"
slug: mold-sccache-advanced
category: ["Instrumente", "Optimizare"]
tags: [rust, mold, sccache, linker, cache, compilare, performanță]
difficulty: advanced
related_concepts: [rustc, LLVM, LTO, incremental-compilation]
reading_time: 12
summary: "mold și sccache sunt două unelte esențiale pentru accelerarea compilării Rust. mold înlocuiește linker-ul implicit și face linkuirea de 5-10× mai rapidă, iar sccache cache-uiește compilările la nivel de crate, reducând rebuild-urile de la 10 minute la sub 1 minut. Articolul prezintă arhitectura, configurarea și benchmark-uri reale."
---

# 🔥 mold + sccache — Accelerare extremă a compilării Rust

> **TL;DR:** `mold` înlocuiește linker-ul implicit (GNU ld) și accelerează linkuirea de 5-10×. `sccache` cache-uiește compilările la nivel de crate și le reutilizează între proiecte și branch-uri. Împreună, transformă un build de 10 minute într-unul de 30-60 de secunde.

---

## 1. De ce compilarea Rust e lentă?

Rust are trei faze principale de compilare:

```
┌─────────────┐     ┌──────────────┐     ┌──────────┐
│  Frontend   │ ──▶ │  LLVM IR     │ ──▶ │ Linkuire │
│ (typecheck, │     │  (optimizări)│     │ (linker) │
│  monomorf.) │     │              │     │          │
└─────────────┘     └──────────────┘     └──────────┘
    ~30% timp           ~40% timp          ~30% timp
```

Marea problemă: **linkuirea** cu `GNU ld` sau `gold` e serială, NU se paralelizează, și pentru un binar Rust de 150MB (debug) poate dura 5-10 secunde. Iar **monomorfizarea generics-urilor** face ca fiecare crate să fie recompilat integral chiar și la o schimbare minoră.

---

## 2. mold — Linker-ul care zboară

### 2.1 Ce face?

`mold` e un linker modern scris de Rui Ueyama (același autor al `lld`). Înlocuiește `ld.bfd` (GNU ld) sau `gold` și face linkuirea **de 5-10× mai repede**.

### 2.2 Cum e posibil?

| Linker | Limbaj | Strategie | Timp (binar 150MB) |
|---|---|---|---|
| GNU ld (bfd) | C | Monolitic, single-thread | ~8-12s |
| gold | C++ | Single-thread, mai rapid | ~5-8s |
| **mold** | **C++** | **Paralel, algoritmi moderni** | **~0.5-1.5s** |

`mold` folosește:
- **Threading** masiv — procesează secțiunile ELF în paralel
- **Hash tables** optimizate — string deduplication O(n)
- **Citire directă** a fișierelor obiect cu `mmap` — zero copy
- **Algoritmi liniari** acolo unde `ld` folosește pătratici

### 2.3 Configurare

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Asta e tot. `cargo` va folosi `mold` pentru orice linkuire ulterioară.

### 2.4 Limitări

- Doar **Linux x86_64** (ARM e experimental)
- Nu suportă toate feature-urile exotice GNU ld (LTO plugins)
- Pentru `cargo check` — **zero impact** (check nu face linkuire)

> **Regulă:** `mold` ajută la `cargo build` (care face link). La `cargo check` nu vezi diferența.

---

## 3. sccache — Cache distribuit la compilare

### 3.1 Ce face?

`sccache` (Squared Cache) e un `ccache` modern pentru Rust (și C/C++). Interceptează apelurile către `rustc`, face hash la codul sursă + versiunile dependințelor, și dacă găsește un rezultat deja compilat, îl returnează direct — fără a recompila.

### 3.2 Arhitectură

```
┌──────────┐     ┌──────────┐     ┌─────────────┐
│  cargo   │ ──▶ │ sccache  │ ──▶ │   rustc     │
│          │     │ (cache)  │     │             │
└──────────┘     └────┬─────┘     └─────────────┘
                      │
               ┌──────┴──────┐
               │   Cache     │
               │ (local FS /  │
               │  S3/GCS)    │
               └─────────────┘
```

### 3.3 Cum funcționează hash-ul

`sccache` face hash SHA-256 la:

1. **Codul sursă** al crate-ului
2. **Toate dependințele** (versiuni exacte)
3. **Compiler flags** (`-C`, `--cfg`, features)
4. **Toolchain** (versiunea de `rustc`)
5. **Target triple** (x86_64-unknown-linux-gnu etc.)

Dacă hash-ul se potrivește, rezultatul `.rlib` e copiat din cache — zero compilare.

### 3.4 Când e util cu adevărat

| Scenariu | Fără sccache | Cu sccache |
|---|---|---|
| `cargo clean` + `cargo build` | 10 min | **~30s** (toate crate-urile din cache) |
| `git stash` + `git stash pop` | 3-5 min | **~1s** |
| Alternare între branch-uri | recompilă tot | **zero** (același hash) |
| CI/CD (build nou de la zero) | 10 min | **~30s** |
| Modificare 1 fișier | ~3s | ~0.5s |

### 3.5 Configurare

```toml
# .cargo/config.toml
[env]
RUSTC_WRAPPER = "sccache"

# Opțional — dimensiune cache
export SCCACHE_CACHE_SIZE="2G"       # default 10G
export SCCACHE_DIR="$HOME/.cache/sccache"
```

### 3.6 Comenzi utile

```bash
# Stare cache
sccache --show-stats

# Golește cache
sccache --clear

# Verifică dacă e activ
sccache --start-server
```

---

## 4. Cum interacționează mold + sccache?

Sunt **complementari** — fiecare acționează pe o fază diferită:

```
sccache                        mold
   │                            │
   ▼                            ▼
┌──────┐   ┌─────────┐   ┌──────────┐   ┌──────────┐
│cargo │──▶│ rustc   │──▶│ object  │──▶│  mold    │──▶ binar
│build │   │(skippat │   │ files   │   │ (rapid)  │
│      │   │ de cache│   │ .rlib   │   │          │
└──────┘   └─────────┘   └──────────┘   └──────────┘
              ▲                              ▲
         sccache skip                   linkuire 5×
         ~80% din timp                  mai rapidă
```

**Efect combinat:**

| Fără niciunul | Doar mold | Doar sccache | Ambele |
|---|---|---|---|
| 10 min | ~9 min | ~30s | **~25-30s** |

Pentru `cargo build` de la zero (clean), `sccache` face toată diferența. Pentru build-uri incrementale (o singură modificare), `mold` ajută la linkuirea finală.

---

## 5. Benchmark real (shop-mvp, Rust + Axum + SQLx)

### 5.1 Mediu de test

- CPU: Intel (4 nuclee / 8 thread-uri)
- RAM: 8 GB DDR4
- SSD: NVMe ~500MB/s citire
- Proiect: 8 lib-uri LEGO + binar shop-mvp (~150MB debug)
- Dependențe: 250+ crate-uri (axum, tokio, sqlx, stripe)

### 5.2 Rezultate

| Operație | Înainte | După | Câștig |
|---|---|---|---|
| `cargo check` (cache rece) | 3.43s | 48s* | primul build |
| `cargo check` (cache cald) | 3.43s | **0.43s** | **8×** |
| `cargo build` (clean) | ~10 min | ~48s | **12.5×** |
| `cargo build` (1 fișier modificat) | ~30s | **~2s** | **15×** |

\* primul run după clean e mai lent pentru că `sccache` popula cache-ul și `mold` nu ajută la `check`.

### 5.3 Interpretare

```
Înainte:  ██████████████████████████████  10 min
După:     ██░░░░░░░░░░░░░░░░░░░░░░░░░░░  48s
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
          sccache reutilizează 250+ crate-uri
          doar 5 crates (libs + binar) se compilează
```

---

## 6. Când NU folosi?

| Situație | Motiv |
|---|---|
| **Cross-compilare** | `mold` nu suportă target-uri diferite de gazdă |
| **macOS** | `mold` nu merge pe macOS (folosește `ld64`/`lld`) |
| **Windows** | `mold` doar Linux |
| **Proiecte mici** (<50 crate-uri) | Câștigul e marginal, setup-ul nu merită |
| **`cargo check` frecvent** | `mold` nu ajută la check; sccache ajută puțin |

---

## 7. Concluzie

| Unealtă | Câștig | Efort instalare | Când contează |
|---|---|---|---|
| **mold** | 5-10× la linkuire | ~1 min | Build-uri cu multe dependențe |
| **sccache** | 10-50× la rebuild | ~10 min | Clean builds, CI/CD, branch-uri multiple |
| **Ambele** | 10-50× global | ~11 min | Proiecte Rust mari / workspace-uri |

Pentru orice proiect Rust real, **ambele sunt must-have**. Nu doar pentru confortul dezvoltatorului, ci și pentru costuri CI/CD — un build care durează 30s în loc de 10 min înseamnă facturi mult mai mici la cloud.

### Resurse

- [mold — GitHub](https://github.com/rui314/mold)
- [sccache — GitHub](https://github.com/mozilla/sccache)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
