use u32_i32_converter::{u32_to_i32, i32_to_u32, PgU32};

fn main() {
    println!("=== DEMO: u32 ↔ i32 Converter ===\n");

    // --------------------------------------------------
    // 1. Conversii simple (funcții libere)
    // --------------------------------------------------
    println!("1. Conversii simple:");
    let original: u32 = 4_000_000_000;
    let stored: i32 = u32_to_i32(original);
    let recovered: u32 = i32_to_u32(stored);
    println!("   Original:  {}", original);
    println!("   Stocat:    {} (PostgreSQL INTEGER)", stored);
    println!("   Recuperat: {}", recovered);
    assert_eq!(original, recovered);
    println!("   ✅ Roundtrip reușit!\n");

    // --------------------------------------------------
    // 2. Toate limitele importante
    // --------------------------------------------------
    println!("2. Limite importante:");
    println!("   u32(0)            → i32({})", u32_to_i32(0));
    println!("   u32(2_147_483_648) → i32({})", u32_to_i32(2_147_483_648));
    println!("   u32(4_294_967_295) → i32({})", u32_to_i32(u32::MAX));
    println!("   i32({}) → u32({})", i32::MIN, i32_to_u32(i32::MIN));
    println!("   i32(0)             → u32({})", i32_to_u32(0));
    println!("   i32({})  → u32({})\n", i32::MAX, i32_to_u32(i32::MAX));

    // --------------------------------------------------
    // 3. Tipul PgU32 (type-safe wrapper)
    // --------------------------------------------------
    println!("3. Tipul PgU32 (wrapper type-safe):");
    let id = PgU32::new(42);
    let pg_value: i32 = id.to_pg_i32();
    let recovered = PgU32::from_pg_i32(pg_value);
    println!("   Original:  {:?}", id);
    println!("   PostgreSQL: {}", pg_value);
    println!("   Recuperat: {:?}", recovered);
    assert_eq!(id, recovered);
    println!("   ✅ PgU32 roundtrip reușit!\n");

    // --------------------------------------------------
    // 4. Conversii From (ergonomie)
    // --------------------------------------------------
    println!("4. Conversii From pentru ergonomie:");
    let from_u32: PgU32 = 100u32.into();
    let back_to_u32: u32 = from_u32.into();
    println!("   u32 → PgU32: {:?}", from_u32);
    println!("   PgU32 → u32: {}", back_to_u32);

    let from_i32: PgU32 = (-100i32).into();
    let back_to_i32: i32 = from_i32.into();
    println!("   i32 → PgU32: {:?}", from_i32);
    println!("   PgU32 → i32: {}\n", back_to_i32);

    // --------------------------------------------------
    // 5. Simulare PostgreSQL cu secvență
    // --------------------------------------------------
    println!("5. Simulare auto-increment cu secvență PostgreSQL:");
    println!("   CREATE SEQUENCE seq START WITH -2147483648 INCREMENT BY 1;");
    let mut next_val: i32 = i32::MIN;  // -2147483648
    for n in 0..5 {
        let id_u32 = i32_to_u32(next_val);
        println!("   INSERT → id={} (PostgreSQL: {})", id_u32, next_val);
        next_val += 1;
        let _ = n;  // suprimă warning-ul de variabilă nefolosită
    }
    println!("   ... după 4 miliarde de inserții ...");
    let ultimul: i32 = i32::MAX;
    println!("   INSERT → id={} (PostgreSQL: {})\n", i32_to_u32(ultimul), ultimul);

    // --------------------------------------------------
    // 6. Exemplu practic cu SQLx (pseudo-cod)
    // --------------------------------------------------
    println!("6. Exemplu practic cu SQLx:");
    println!("   // Inserare");
    println!("   sqlx::query(\"INSERT INTO produse (id, nume) VALUES ($1, $2)\")");
    println!("       .bind(PgU32::new(42).to_pg_i32())");
    println!("       .bind(\"Laptop\")");
    println!("       .execute(&pool).await?;");
    println!();
    println!("   // Citire");
    println!("   let row: (i32, String) = sqlx::query_as(...)");
    println!("   let id = PgU32::from_pg_i32(row.0);");
    // Am corectat aici: am scăpat acoladele cu {{ și }}
    println!("   println!(\"id={{}}\", id.get());  // 42\n");

    println!("=== Toate testele au trecut! ✅ ===");
}
