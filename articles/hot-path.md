---
title: "Hot Path — optimizarea caii fierbinti in sisteme"
slug: "hot-path"
summary: "Hot path (calea fierbinte) e sectiunea de cod executata cel mai frecvent. Identificarea si optimizarea ei e cea mai eficienta strategie de optimizare — 90% din timp e petrecut in 10% din cod. Principiul lui Pareto aplicat la performanta."
category_path: ["Sisteme și arhitectură", "Optimizare"]
tags: ["hot path", "optimizare", "performanta", "Pareto", "profiling", "cache", "branch prediction"]
difficulty: "advanced"
reading_time: 18
related: ["structuri-de-date", "memory-wall", "big-o", "data-oriented-design", "sisteme-de-operare-concepte"]
---

## Ce e hot path?

Orice sistem software are o **cale de executie dominanta** — secventa de instructiuni executata de cele mai multe ori. Asta e hot path-ul. Restul codului e cold path.

**Regula 90/10:** 90% din timpul de executie e petrecut in 10% din cod. A optimiza cold path-ul e aproape intotdeauna o pierdere de vreme.

## Cum identifici hot path-ul

Fara instrumente, ghicesti gresit. Studiile arata ca programatorii identifica corect bottleneck-ul doar in ~30% din cazuri.

**Instrumente:**
- **Profiling sampling:** La fiecare ms, inregistreaza instructiunea curenta. La final, ai o distributie statistica. Folosit de perf (Linux), Instruments (macOS), VTune (Intel).
- **Instrumentare:** Adauga contoare la intrarea/iesirea din functii. Mai exact, dar overhead mai mare (Valgrind, Callgrind).
- **Event counters:** CPU-ul are contoare hardware pentru cache misses, branch mispredictions, TLB misses (perf stat, VTune).

## Optimizarea hot path-ului

Odata identificat, strategiile de optimizare sunt:

### 1. Eliminarea codului mort
Daca o ramura nu e aproape niciodata executata, mut-o in cold path. Ex: validarea parametrilor in functii apelate de milioane de ori.

### 2. Inlining
Elimina overhead-ul apelului de functie. Compilatoarele fac asta automat pentru functii mici. `inline` in C++ e doar un sugestie.

### 3. Branch elimination
Inlocuieste branch-urile cu lookup tables sau aritmetica. Ex: in loc de `if (x > 0) return 1; else return 0;` foloseste `return (x > 0)`, care pe majoritatea procesoarelor nu genereaza branch.

### 4. Data-oriented design
Reorganizeaza datele ca accesele in hot path sa fie secventiale, nu random. Cache misses in hot path sunt dezastruase.

### 5. Loop unrolling
Ruleaza 4 iteratii deodata pentru a reduce overhead-ul buclei. Compilatoarele moderne fac asta automat la -O3.

## Cache-ul si hot path-ul

Hot path-ul trebuie sa incapa in **L1 cache** (de obicei 32KB pentru instructiuni, 32KB pentru date). Daca hot path-ul e mai mare decat L1, vei avea cache misses constante, iar performanta se degradeaza dramatic.

**Code layout:** Plaseaza hot path-ul contiguu in memorie, ideal intr-o singura pagina. Functiile din hot path trebuie declarate impreuna.

## Branch prediction si hot path

Procesoarele moderne au predictoare de salturi cu rata de succes >95% pentru hot path-uri bine definite. Pattern-urile neregulate (ex: un if care depinde de date aleatoare) distrug predictia si cauzeaza pipeline flushes.

## Exemplu — Linux kernel

Linux separa explicit hot path de cold path prin macro-urile `likely()` si `unlikely()`:

```c
if (unlikely(!ptr)) return -ENOMEM;
```

Aceste macro-uri spun compilatorului sa plaseze ramura rara in cold path, imbunatatind code locality.

## JIT compilation

Compilatoarele JIT (Java HotSpot, V8) identifica hot path-urile la runtime si le compileaza din nou cu optimizari mai agresive (de la interpretat → C1 → C2). Asta se numeste **adaptive optimization**.

## Cold path

Cold path e restul — initializari, cleanup, erori, cazuri exceptionale. Nu merita optimizat, dar merita mutat din hot path prin:
- Exceptii (nu verifica la fiecare apel, lasa exceptia sa se arunce)
- Lazy initialization (initializeaza doar cand e necesar)
- Deferred cleanup (curata dupa ce ai terminat in hot path)

