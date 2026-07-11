# 📋 Audit mediu dezvoltare — shop-mvp

> Generat: 2026-07-11
> Status: **DEBUG MODE** (alpha, fără utilizatori reali)

---

## ✅ Sistem

| Componentă | Valoare | Optim |
|-----------|---------|-------|
| **OS** | Ubuntu 7.0.0-27-generic | — |
| **Kernel** | Linux 7.0.0 x86_64 | PREEMPT_DYNAMIC |
| **CPU** | AMD FX-8800P (4 core) | `performance` governor |
| **RAM** | 6.7GB total, ~5GB used | 2GB ZRAM swap + 4GB file swap |
| **Disk** | 219GB SSD, 14GB liber (94%) | ⚠️ Aproape plin |
| **Swappiness** | 10 | ✅ |
| **Inotify** | 524288 | ✅ |
| **ZRAM** | 2GB lz4 (comprimare 68%) | ✅ |
| **Mount** | `noatime` | ✅ |

## ✅ Tooling Rust

| Componentă | Versiune | Status |
|-----------|----------|--------|
| **rustc** | 1.96.1 (2024 edition) | ✅ |
| **mold** | 2.40.4 | ✅ Configurat în `.cargo/config.toml` |
| **sccache** | 0.16.0 | ✅ Cache 10GB, wrapper configurat |
| **cargo check** | 0.35s (incremental) | ✅ Rapid |
| **Binary** | 154MB (debug) | ✅ Normal pentru debug |

## ✅ PostgreSQL (Docker)

| Parametru | Valoare | Status |
|-----------|---------|--------|
| **Imagine** | `pgvector/pgvector:pg18` | ✅ |
| **Container** | `dev-postgres` (healthy) | ✅ |
| **shared_buffers** | 512MB | ✅ |
| **random_page_cost** | 1.1 (SSD) | ✅ |
| **work_mem** | 16MB | ✅ |
| **synchronous_commit** | off (debug) | ✅ |
| **DB size** | 11MB | ✅ |
| **Tabele** | 13 (products, users, orders, etc.) | ✅ |

## ✅ Proiect

| Verificare | Status |
|-----------|--------|
| **Compilare** | ✅ `cargo check -p shop-mvp` — 0 warnings |
| **Teste compilare** | ✅ 6 warnings (minore) |
| **Teste DB** | ⏳ Neexecutate (necesită DB + async runtime) |
| **.env** | ✅ Toate variabilele configurate |
| **.cargo/config.toml** | ✅ mold + sccactivate activate |
| **Cargo.toml resolver** | ✅ `resolver = "3"` adăugat |
| **Loguri** | ✅ Păstrată doar sesiunea curentă |

## 🔧 Ce s-a reparat

1. **`.cargo/config.toml`** — copiat de la myapp (mold + sccache)
2. **`Cargo.toml`** — adăugat `resolver = "3"` (eliminat warning)
3. **Teste DB** — schimbate din `#[test]` sync în `#[tokio::test]` async + adăugat `use sqlx::Row;`
4. **Log vechi** — șters `shop-mvp.log.2026-07-10` (~300KB eliberați)
5. **Warnings** — reduse de la 9 la 6 prin `cargo fix`

## ⚠️ Rămase de urmărit

- **Spațiu disk 94%** — `cargo clean` dacă devine critic
- **6 warnings** — `AdminState` fields never read, variabile nefolosite
- **Testele DB** nu au fost executate (doar compilate)

## 📁 Fișiere de configurare

| Fișier | Rol |
|--------|-----|
| `.env` | Variabile de mediu (DB, API keys, debug) |
| `.cargo/config.toml` | mold linker + sccache wrapper |
| `compose.yml` | Docker PostgreSQL cu pgvector |
