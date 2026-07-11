---
title: "Pgvector pe Android/Termux — Rezolvarea problemei de simboluri nelinkate"
slug: "pgvector-android-termux"
summary: "O investigație aprofundată a erorii 'cannot locate symbol acos' la instalarea pgvector pe Android via Termux: de ce apare, cum am depanat-o și cum am rezolvat-o prin legarea corectă a bibliotecii matematice."
category_path: ["Sisteme de operare", "Baze de date", "Android"]
tags: ["pgvector", "Termux", "Android", "PostgreSQL", "vector search", "acos", "shared libraries", "linker", "embedded", "cross-platform"]
difficulty: "advanced"
reading_time: 12
related: ["post-posix", "edge-ai-serverless", "baze-de-date-sql-nosql", "ebpf", "memory-wall"]
---

## Introducere

**Pgvector** este extensia PostgreSQL care adaugă suport pentru căutare vectorială — coloane de tip `vector(n)`, indexuri `IVFFlat` și `HNSW`, similaritate cosine, distanță euclidiană等等. Este esențială pentru aplicații AI/ML care stochează embeddings direct în baza de date.

Instalarea pe Linux standard (x86_64, ARM64) este trivială: `apt install postgresql-vector` sau `make && make install`. Pe **Android via Termux**, însă, lucrurile se complică. Acest articol documentează o eroare întâlnită pe un **Samsung S22 (Exynos 2200, Android 14)** și procesul de depanare până la soluție.

---

## 1. Context

### 1.1 Arhitectura sistemului

| Componentă | Detalii |
|-----------|---------|
| Telefon | Samsung S22 (Exynos 2200, 8 core-uri, 7.1 GB RAM) |
| Sistem | Android 14, Termux |
| PostgreSQL | 18.2, instalat nativ în Termux |
| pgvector | v0.8.5, compilat din sursă |
| Compilator | Clang 18 (Android NDK) — `aarch64-linux-android-clang` |
| Bibliotecă matematică | `libm.so` — parte din Bionic (libc Android) |

### 1.2 Simptomele

După compilarea și instalarea pgvector cu `make && make install`, încercarea de a activa extensia eșuează:

```sql
test=# CREATE EXTENSION IF NOT EXISTS vector;
ERROR:  could not load library "/data/data/com.termux/files/usr/lib/postgresql/vector.so":
dlopen failed: cannot locate symbol "acos" referenced by
"/data/data/com.termux/files/usr/lib/postgresql/vector.so"...
```

Mai mult, aplicația Rust (Axum + SQLx) care rulează pe același telefon crapă la pornire cu aceeași eroare:

```
Error: error returned from database: extension "vector" is not available at line 673
```

Chiar și codificată defensiv (`if let Err(e) = sqlx::query("CREATE EXTENSION IF NOT EXISTS vector").execute(&pool).await { eprintln!("⚠️  pgvector: {e}"); }`), aplicația moare cu `FromResidual` — pentru că eroarea vine din interiorul **migrărilor SQLx**, nu din codul explicit.

---

## 2. Anatomia problemei

### 2.1 Ce este `acos`?

`acos` este funcția **arc-cosinus** din biblioteca matematică standard C (`math.h`). În codul pgvector, este folosită pentru calcule geometrice în indexurile HNSW și IVFFlat:

```c
// src/hnswutils.c (aproximativ)
double angle = acos(cosine_similarity(a, b));
```

Pe Linux (glibc), `acos` face parte din `libm.so`, care este **linkată implicit** pentru orice bibliotecă partajată compilată cu `-lm`.

Pe Android, însă, Bionic (libc Android) include **doar subsetul de bază** al funcțiilor matematice. Cele care depind de hardware float (precum `acos`, `asin`, `atan2` în variantă dublă precizie) sunt opționale și trebuie linkate explicit.

### 2.2 De ce apare eroarea doar pe Android?

| Sistem | libc | Biblioteca matematică | Linkare implicită |
|--------|------|---------------------|-------------------|
| Linux (glibc) | `libc.so.6` | `libm.so.6` | ✅ Da, prin `-lm` în LDFLAGS |
| macOS | `libSystem.dylib` | Încorporată în libSystem | ✅ Da |
| Android (Bionic) | `libc.so` | `libm.so` (separată) | ❌ **Nu** — trebuie linkată explicit |
| Termux (clang) | `libc.so` (Bionic) | `libm.so` | ❌ Makefile pgvector nu adaugă `-lm` |

Makefile-ul pgvector folosește sistemul de build PostgreSQL (PGXS), care implicit nu adaugă `-lm` la linkare. Pe Linux, linkerul o adaugă automat; pe Android, nu.

### 2.3 Verificarea

Confirmarea că simbolul lipsește din biblioteca compilată:

```bash
$ nm -D /data/data/com.termux/files/usr/lib/postgresql/vector.so | grep acos
         U acos
```

Litera `U` înseamnă **undefined** — simbolul nu este rezolvat. La runtime, `dlopen` încearcă să-l găsească și eșuează.

Comparativ, pe un sistem unde funcționează:

```bash
$ nm -D /usr/lib/postgresql/16/lib/vector.so | grep acos
         U acos@@GLIBC_2.17
```

Același simbol, dar linkat corect cu glibc.

---

## 3. Soluția

### 3.1 Recompilarea cu `SHLIB_LINK="-lm"`

Makefile-ul pgvector folosește variabila `SHLIB_LINK` pentru a specifica flagurile de linker. Adăugând `-lm` (linkează biblioteca matematică), simbolul `acos` este rezolvat:

```bash
cd $HOME/pgvector
make clean
make SHLIB_LINK="-lm"
make install
```

Alternativ, se poate seta și prin `PG_CPPFLAGS`:

```bash
make PG_CPPFLAGS="-D_LARGEFILE64_SOURCE -lm"
```

În ambele cazuri, linia de compilare devine:

```
clang -shared -o vector.so ... -lm
```

### 3.2 Verificarea soluției

```bash
$ psql -d test -c "CREATE EXTENSION IF NOT EXISTS vector;"
CREATE EXTENSION

$ psql -d test -c "SELECT typname FROM pg_type WHERE typname='vector';"
 typname
---------
 vector
(1 row)
```

Și în aplicația Rust:

```
2026-07-09T19:49:55.247608Z  INFO sqlx::postgres::notice: extension "vector" already exists, skipping
✅ pgvector activat
```

### 3.3 Automatizarea

Pentru reproductibilitate, comanda completă de build:

```bash
git clone --depth 1 https://github.com/pgvector/pgvector.git
cd pgvector
make clean 2>/dev/null
make SHLIB_LINK="-lm"
make install
psql -d test -c "CREATE EXTENSION IF NOT EXISTS vector;"
```

---

## 4. Lecții învățate

### 4.1 Diferențe între glibc și Bionic

Bionic (libc Android) este **semnificativ mai minimalistă** decât glibc. Proiectată pentru embedded/mobile, ea expune doar ce este strict necesar. Funcțiile matematice avansate (`acos`, `asin`, `atan2`, `log`, `exp` în dublă precizie) sunt în `libm.so`, care **nu** este linkată implicit.

Pentru dezvoltatorii de biblioteci C/C++ care țintesc Android:
- **Nu presupuneți** că `-lm` este implicit
- Verificați simbolurile cu `nm -D`
- Testați `dlopen` în medii embedded

### 4.2 Debugging biblioteci partajate

Când o bibliotecă partajată (`*.so`) nu se încarcă:

```bash
# 1. Verifică simbolurile nerezolvate
nm -D library.so | grep " U "

# 2. Verifică dependințele
readelf -d library.so | grep NEEDED

# 3. Verifică arhitectura
file library.so

# 4. Testează dlopen manual
echo 'dlopen("library.so", RTLD_NOW)' | python3
```

### 4.3 Implicații pentru cross-compilare

Aceeași problemă apare la **cross-compilare** (x86_64 → ARM64) dacă linkerul cross nu este configurat corect. Soluția: adăugați `-lm` explicit în LDFLAGS.

Pentru ecosistemul Rust (unde acest caz a fost întâlnit):
- `cc` crate — verificați că `-lm` este în `cargo:rustc-link-lib=m`
- SQLx — migrările compilează SQL în binar; erorile de runtime pot veni din SQL încorporat, nu din codul explicit
- Cross-compilare pentru Android — folosiți NDK-ul corect și verificați cu `aarch64-linux-android-clang`

---

## 5. Concluzie

Problema `cannot locate symbol "acos"` la pgvector pe Android/Termux este cauzată de **lipsa linkării explicite a bibliotecii matematice** (`-lm`). Pe Linux/glibc acest flag este implicit; pe Android/Bionic nu.

Soluția este simplă: recompilați pgvector cu `make SHLIB_LINK="-lm"`. Dincolo de soluția imediată, acest caz ilustrează diferențe fine dar critice între platformele Unix, importanța verificării simbolurilor în biblioteci partajate și provocările dezvoltării cross-platform în medii embedded/mobile.

Pentru dezvoltatorii de aplicații Rust pe ARM64 Android, lecția principală: **nu presupuneți nimic despre platforma țintă** — verificați, testați și documentați fiecare dependență nativă.

---

## Anexă: Comenzi utile

```bash
# Listare simboluri nerezolvate
nm -D vector.so | grep " U " | head -20

# Listare dependințe bibliotecă
readelf -d vector.so | grep NEEDED

# Verificare tip vector în PostgreSQL
psql -d test -c "SELECT oid, typname, typlen FROM pg_type WHERE typname LIKE '%vec%';"

# Testare dlopen manual
python3 -c "import ctypes; lib = ctypes.CDLL('/data/data/com.termux/files/usr/lib/postgresql/vector.so'); print('OK')"

# Rebuild complet pgvector
cd ~/pgvector && git pull && make clean && make SHLIB_LINK="-lm" && make install
```
