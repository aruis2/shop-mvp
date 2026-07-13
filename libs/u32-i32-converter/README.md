# u32-i32-converter

Conversii **zero-cost** între `u32` și `i32` pentru a stoca **4 miliarde de valori** în PostgreSQL folosind doar **4 bytes** (INTEGER).

[![Crates.io](https://img.shields.io/crates/v/u32-i32-converter)](https://crates.io/crates/u32-i32-converter)
[![License](https://img.shields.io/crates/l/u32-i32-converter)](https://github.com/tau/u32-i32-converter)

---

## 🎯 Problema

PostgreSQL nu are tipuri unsigned. `INTEGER` este `i32` și stochează doar **~2 miliarde** de valori (0 la 2.147.483.647). 

Dar tu ai nevoie de **4 miliarde** (0 la 4.294.967.295) și vrei să folosești doar 4 bytes, nu 8 (`BIGINT`).

## ✅ Soluția

O conversie matematică simplă care shiftează domeniul:

| PostgreSQL (i32) | Rust (u32) |
|------------------|------------|
| -2.147.483.648   | 0          |
| 0                | 2.147.483.648 |
| 2.147.483.647    | 4.294.967.295 |

**Zero overhead la runtime:** o singură instrucțiune de adunare/scădere.

---

## 📦 Instalare

```toml
[dependencies]
u32-i32-converter = "0.1"
Zero dependințe externe. Doar biblioteca standard Rust.

🚀 Utilizare rapidă
Funcții simple
rust
use u32_i32_converter::{u32_to_i32, i32_to_u32};

let original: u32 = 4_000_000_000;
let stored: i32 = u32_to_i32(original);   // pentru PostgreSQL
let recovered: u32 = i32_to_u32(stored);  // înapoi în Rust

assert_eq!(original, recovered);
Tip type-safe PgU32
rust
use u32_i32_converter::PgU32;

// Creezi un ID
let id = PgU32::new(42);

// Pentru PostgreSQL
let pg_value: i32 = id.to_pg_i32();

// După citirea din PostgreSQL
let recovered = PgU32::from_pg_i32(pg_value);

assert_eq!(id, recovered);
Conversii From (ergonomie)
rust
use u32_i32_converter::PgU32;

let id: PgU32 = 100u32.into();    // din u32
let back: u32 = id.into();        // înapoi la u32

let id: PgU32 = (-1i32).into();   // din i32 (PostgreSQL)
let back: i32 = id.into();        // înapoi la i32
Cu SQLx
rust
use u32_i32_converter::PgU32;

// Inserare
let id = PgU32::new(42);
sqlx::query("INSERT INTO produse (id, nume) VALUES ($1, $2)")
    .bind(id.to_pg_i32())  // se duce ca i32 în PostgreSQL
    .bind("Laptop")
    .execute(&pool)
    .await?;

// Citire
struct Row { id: i32, nume: String }
let row = sqlx::query_as::<_, Row>("SELECT id, nume FROM produse")
    .fetch_one(&pool)
    .await?;

let id = PgU32::from_pg_i32(row.id);
println!("{}", id.get());  // 42
🗄️ Secvență PostgreSQL pentru auto-increment
sql
-- Creezi o secvență care folosește tot domeniul de 4 bytes
CREATE SEQUENCE produse_id_seq
    START WITH -2147483648   -- echivalent u32 = 0
    INCREMENT BY 1
    MINVALUE -2147483648
    MAXVALUE 2147483647;

-- O folosești în tabel
CREATE TABLE produse (
    id INTEGER PRIMARY KEY DEFAULT nextval('produse_id_seq'),
    nume TEXT NOT NULL
);
Primele inserții:

id (PostgreSQL)	id (u32)	nume
-2147483648	0	Laptop
-2147483647	1	Telefon
-2147483646	2	Tabletă
📊 Performanță
Operație	Cost
u32_to_i32()	1 instrucțiune CPU
i32_to_u32()	1 instrucțiune CPU
PgU32::new()	0 cicli (optimizat de compilator)
Toate funcțiile sunt #[inline(always)] – compilatorul le încorporează direct, fără niciun apel de funcție.

🧪 Teste
bash
cargo test
Teste incluse:

✅ Toate limitele (0, u32::MAX, i32::MIN, i32::MAX)

✅ Roundtrip pentru valori cheie

✅ Fără coliziuni (~100k valori verificate)

✅ Conversii From<u32> și From<i32>

✅ Type safety cu PgU32

🆚 Alternative
Soluție	Bytes	Valori	Dependințe
SERIAL (i32)	4	2 miliarde	-
BIGSERIAL (i64)	8	9 chintilioane	-
UUID	16	enorm	-
u32-i32-converter	4	4 miliarde	zero
🤝 Contribuții
Contribuțiile sunt binevenite! Deschide un issue sau un pull request.

📜 Licență
MIT OR Apache-2.0

✨ Autor
Creat cu 🦀 de Iuri, cu asistență de la DeepSeek AI (pair programming, 2026).

Această bibliotecă a fost scrisă în timpul unei sesiuni live de programare,
ca răspuns la nevoia de a stoca 4 miliarde de ID-uri în PostgreSQL
fără a risipa 4 bytes per rând cu BIGINT.

📚 Documentație
Docs.rs

Crates.io

Repository

Construit cu ❤️ și Rust 🦀