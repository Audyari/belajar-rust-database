use chrono::DateTime;
use chrono::{FixedOffset, Utc};
use futures::TryStreamExt;
use sqlx::FromRow;
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use std::time::Instant;

struct Category {
    id: String,
    name: String,
    description: String,
}

struct Todos {
    id: i32,
    task: String,
    completed: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow, Debug)]
struct Brands {
    id: String,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>, // ← ganti NaiveDateTime → DateTime<Utc>
    updated_at: DateTime<Utc>, // ← ganti NaiveDateTime → DateTime<Utc>
}

#[derive(Debug, FromRow)]
struct Bus {
    id: i32,
    name: String,
}

async fn get_pool() -> Result<PgPool, sqlx::Error> {
    let url = "postgresql://postgres:123456@localhost:5432/belajar_rust_database";

    PgPoolOptions::new()
        .max_connections(5) // ← 5 untuk laptop 2 core
        // .max_connections(6)  // ← 6 masih aman
        // .max_connections(8)  // ← 8 mulai berat
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(3)) //connection akan ditutup jika tidak digunakan selama 3 detik
        .idle_timeout(Duration::from_secs(3)) //connection akan ditutup jika tidak digunakan selama 3 detik
        .connect(url)
        .await
}

fn print_current_time() {
    let now_utc = Utc::now();
    println!("Waktu: {}", now_utc.format("%Y-%m-%d %H:%M:%S"));
}

async fn insert_category(
    pool: &PgPool,
    id: &str,
    name: &str,
    description: Option<String>,
) -> Result<(), sqlx::Error> {
    // Cek apakah ID sudah ada sekaligus ambil datanya
    let existing = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT name, description FROM category WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    if let Some((existing_name, existing_desc)) = existing {
        println!("⚠️  Category dengan ID '{id}' sudah ADA, tidak jadi insert");
        println!("   Data di database:");
        println!("     ID          : {id}");
        println!("     Name        : {existing_name}");
        println!("     Description : {existing_desc:?}");
        println!("   Data yang ingin diinsert:");
        println!("     Name        : {name}");
        println!("     Description : {description:?}");
        return Ok(());
    }

    // Insert jika belum ada
    sqlx::query("INSERT INTO category (id, name, description) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(description)
        .execute(pool)
        .await?;

    println!("✅ Category '{name}' (ID: {id}) berhasil ditambahkan");
    Ok(())
}

async fn insert_category_with_tx(
    pool: &PgPool,
    id: &str,
    name: &str,
    description: Option<String>,
) -> Result<(), sqlx::Error> {
    // Mulai transaction
    let mut tx = pool.begin().await?;

    // Cek apakah ID sudah ada (dalam transaction)
    let existing = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT name, description FROM category WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx) // ← pakai &mut *tx, BUKAN pool!
    .await?;

    if let Some((existing_name, existing_desc)) = existing {
        println!("⚠️  Category dengan ID '{id}' sudah ADA, tidak jadi insert");
        println!("   Data di database:");
        println!("     ID          : {id}");
        println!("     Name        : {existing_name}");
        println!("     Description : {existing_desc:?}");
        println!("   Data yang ingin diinsert:");
        println!("     Name        : {name}");
        println!("     Description : {description:?}");

        // Rollback transaction (batalkan semua operasi)
        tx.rollback().await?;
        return Ok(());
    }

    // Insert jika belum ada (dalam transaction yang sama)
    sqlx::query("INSERT INTO category (id, name, description) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(description)
        .execute(&mut *tx) // ← pakai tx, BUKAN pool!
        .await?;

    // Commit transaction (simpan ke database)
    tx.commit().await?;

    println!("✅ Category '{name}' (ID: {id}) berhasil ditambahkan");
    Ok(())
}

async fn insert_todo(pool: &PgPool, task: &str, completed: bool) -> Result<(), sqlx::Error> {
    // Cek apakah task dengan nama yang sama sudah ada
    let row = sqlx::query("SELECT EXISTS (SELECT 1 FROM todos WHERE task = $1) AS exists")
        .bind(task)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = row {
        let exists = row.get("exists");
        if exists {
            println!("⚠️  Task '{task}' sudah ADA, tidak jadi insert");
            return Ok(());
        }
    }

    // Insert jika belum ada
    let now_utc = Utc::now(); // Simpan di database sebagai UTC

    // Konversi ke WIB untuk ditampilkan (opsional, hanya untuk log)
    let wib = FixedOffset::east_opt(7 * 3600).unwrap();
    let now_wib = now_utc.with_timezone(&wib);

    sqlx::query("INSERT INTO todos (task, completed, created_at) VALUES ($1, $2, $3)")
        .bind(task)
        .bind(completed)
        .bind(now_utc) // Tetap bind UTC ke database
        .execute(pool)
        .await?;

    // Tampilkan waktu dalam WIB
    println!(
        "✅ Task '{task}' berhasil ditambahkan pada {}",
        now_wib.format("%Y-%m-%d %H:%M:%S WIB")
    );
    Ok(())
}

async fn get_todo(pool: &PgPool, id: i32) -> Result<Option<Todos>, sqlx::Error> {
    // Ganti query_as! → query (tanpa macro)
    let row = sqlx::query("SELECT id, task, completed, created_at FROM todos WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let todo = row.map(|r| Todos {
        id: r.get("id"),
        task: r.get("task"),
        completed: r.get("completed"),
        created_at: r.get("created_at"),
    });

    Ok(todo)
}

#[allow(dead_code)]
async fn get_all_todos(pool: &PgPool) -> Result<Vec<Todos>, sqlx::Error> {
    let start = Instant::now();

    let rows = sqlx::query("SELECT id, task, completed, created_at FROM todos ORDER BY id")
        .fetch_all(pool)
        .await?;

    println!("⏱️ fetch_all() selesai dalam {:?}", start.elapsed()); // ← 100 detik!

    let mut todos = Vec::new();
    for row in rows {
        todos.push(Todos {
            id: row.get("id"),
            task: row.get("task"),
            completed: row.get("completed"),
            created_at: row.get("created_at"),
        });
    }

    println!("⏱️ Mapping selesai dalam {:?}", start.elapsed());
    Ok(todos)
}

#[allow(dead_code)]
async fn process_todos_streaming<F>(pool: &PgPool, mut callback: F) -> Result<(), sqlx::Error>
where
    F: FnMut(Todos),
{
    let mut stream =
        sqlx::query("SELECT id, task, completed, created_at FROM todos ORDER BY id").fetch(pool);

    while let Some(row) = stream.try_next().await? {
        let todo = Todos {
            id: row.get("id"),
            task: row.get("task"),
            completed: row.get("completed"),
            created_at: row.get("created_at"),
        };

        callback(todo); // ← LANGSUNG proses (cetak, dll)
        // Tidak disimpan di vector!
    }

    Ok(())
}
async fn get_category(pool: &PgPool, id: &str) -> Result<Option<Category>, sqlx::Error> {
    // Ganti query_as! (macro) → query_as (function)
    let row = sqlx::query("SELECT id, name, description FROM category WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    // MAPPING DI LUAR (match)
    let category = row.map(|r| Category {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
    });

    Ok(category)
}

async fn get_all_category(pool: &PgPool) -> Result<Vec<Category>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, name, description FROM category")
        .fetch_all(pool)
        .await?;

    // MAPPING DI LUAR (loop manual)
    let mut categories = Vec::new();
    for row in rows {
        categories.push(Category {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
        });
    }

    Ok(categories)
}

async fn insert_brands(
    pool: &PgPool,
    id: &str,
    name: &str,
    description: Option<String>,
) -> Result<(), sqlx::Error> {
    // Cek apakah ID sudah ada
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM brands WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await?;

    if exists.0 {
        println!("⚠️  Brand dengan ID '{id}' sudah ADA, tidak jadi insert");
        return Ok(());
    }

    // Insert jika belum ada
    sqlx::query("INSERT INTO brands (id, name, description) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(name)
        .bind(description)
        .execute(pool)
        .await?;

    println!("✅ Brand '{}' (ID: {}) berhasil ditambahkan", name, id);
    Ok(())
}

async fn get_brand_by_id(pool: &PgPool, id: &str) -> Result<Option<Brands>, sqlx::Error> {
    let brand = sqlx::query_as::<_, Brands>(
        "SELECT id, name, description, created_at, updated_at FROM brands WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(brand) // ← langsung Option<Brands>!
}

async fn get_all_brands(pool: &PgPool) -> Result<Vec<Brands>, sqlx::Error> {
    let brands = sqlx::query_as::<_, Brands>(
        "SELECT id, name, description, created_at, updated_at FROM brands ORDER BY id",
    )
    .fetch_all(pool)
    .await?;

    Ok(brands) // ← LANGSUNG! tanpa manual mapping!
}

//insert bus
async fn insert_bus(pool: &PgPool, name: &str) -> Result<(), sqlx::Error> {
    match sqlx::query("INSERT INTO buses (name) VALUES ($1)")
        .bind(name)
        .execute(pool)
        .await
    {
        Ok(_) => println!("✅ Bus '{}' berhasil ditambahkan", name),
        Err(e) => {
            if let Some(db_err) = e.as_database_error() {
                if db_err.code().as_deref() == Some("23505") {
                    // Unique violation
                    println!("⚠️  Bus '{}' sudah ADA, skip insert", name);
                    return Ok(());
                }
            }
            return Err(e);
        }
    }
    Ok(())
}
async fn get_all_buses(pool: &PgPool) -> Result<Vec<Bus>, sqlx::Error> {
    let buses = sqlx::query_as::<_, Bus>("SELECT id, name FROM buses ORDER BY id")
        .fetch_all(pool)
        .await?;

    Ok(buses)
}

async fn get_bus_by_id(pool: &PgPool, id: i32) -> Result<Option<Bus>, sqlx::Error> {
    let bus = sqlx::query_as::<_, Bus>("SELECT id, name FROM buses WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(bus)
}

fn main() -> Result<(), sqlx::Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let pool = get_pool().await?;
        println!("✅ Koneksi berhasil!");

        print_current_time();

        insert_category(
            &pool,
            "C001",
            "Elektronik",
            Some("Produk elektronik seperti laptop, hp, tv".to_string()),
        )
        .await?;

        insert_category(
            &pool,
            "C002",
            "Pakaian",
            Some("Produk pakaian seperti baju, celana, rok".to_string()),
        )
        .await?;

        insert_category(
            &pool,
            "C003",
            "Makanan",
            Some("Produk makanan seperti baju, celana, rok".to_string()),
        )
        .await?;

        //insert category with transaction
        println!("\n=== INSERT CATEGORY WITH TRANSACTION ===");

        insert_category_with_tx(
            &pool,
            "C004",
            "Minuman",
            Some("Produk minuman seperti kopi, teh, susu".to_string()),
        )
        .await?;

        // ambil data yang ada
        println!("\n=== GET CATEGORY BY ID ===");
        match get_category(&pool, "C001").await? {
            Some(cat) => println!(
                "✅ ID: {}, Name: {}, Description: {:?}",
                cat.id, cat.name, cat.description
            ),
            None => println!("⚠️ ID C001 tidak ditemukan"),
        }

        // ambil semua data
        let all_categories = get_all_category(&pool).await?;
        println!("✅ Semua data category:");
        for cat in all_categories {
            println!(
                "✅ ID: {}, Name: {}, Description: {:?}",
                cat.id, cat.name, cat.description
            );
        }

        //insert todo
        insert_todo(&pool, "Membeli makan", false).await?;
        //insert todo
        insert_todo(&pool, "Menjual pulsa", false).await?;
        //insert todo
        insert_todo(&pool, "Membeli buku", false).await?;

        //ambil data todo
        match get_todo(&pool, 1).await? {
            Some(todo) => {
                // Konversi UTC ke WIB
                let wib = chrono::FixedOffset::east_opt(7 * 3600).unwrap();
                let created_wib = todo.created_at.with_timezone(&wib);

                println!(
                    "✅ ID: {}, Task: {}, Completed: {}, Created At: {} WIB",
                    todo.id,
                    todo.task,
                    todo.completed,
                    created_wib.format("%Y-%m-%d %H:%M:%S")
                );
            }
            None => println!("⚠️ ID 1 tidak ditemukan"),
        }

        //ambil semua data todo

        // println!("✅ Semua data todo Menggunakan Fetch All");
        // let all_todos = get_all_todos(&pool).await?;

        // ⚠️ WARNING jika data kosong
        // if all_todos.is_empty() {
        //     println!("⚠️ WARNING: Tidak ada data todo di database!");
        //     println!("   Silakan tambahkan data terlebih dahulu.");
        // } else {
        //     println!("✅ {} data todo ditemukan:", all_todos.len());
        //     for todo in all_todos {
        //         let wib = chrono::FixedOffset::east_opt(7 * 3600).unwrap();
        //         let created_wib = todo.created_at.with_timezone(&wib);
        //         println!(
        //             "   ID: {}, Task: {}, Completed: {}, Created At: {} WIB",
        //             todo.id,
        //             todo.task,
        //             todo.completed,
        //             created_wib.format("%Y-%m-%d %H:%M:%S")
        //         );
        //     }
        // }

        //get_all_todos_streaming

        // println!("✅ Semua data todo Menggunakan Stream");

        // process_todos_streaming(&pool, |todo| {
        //     let wib = chrono::FixedOffset::east_opt(7 * 3600).unwrap();
        //     let created_wib = todo.created_at.with_timezone(&wib);
        //     println!(
        //         "✅ ID: {}, Task: {}, Completed: {}, Created At: {} WIB",
        //         todo.id,
        //         todo.task,
        //         todo.completed,
        //         created_wib.format("%Y-%m-%d %H:%M:%S")
        //     );
        // })
        // .await?;

        //1. insert brands
        insert_brands(
            &pool,
            "B001",
            "Nike",
            Some("Produk sepatu Nike".to_string()),
        )
        .await?;
        insert_brands(
            &pool,
            "B002",
            "Adidas",
            Some("Produk sepatu Adidas".to_string()),
        )
        .await?;
        insert_brands(
            &pool,
            "B003",
            "Puma",
            Some("Produk sepatu Puma".to_string()),
        )
        .await?;
        insert_brands(&pool, "B004", "Generic", None).await?; // None tanpa .to_string()

        // get brand by id
        match get_brand_by_id(&pool, "B001").await? {
            Some(brand) => {
                let wib = chrono::FixedOffset::east_opt(7 * 3600).unwrap();
                let created_wib = brand.created_at.with_timezone(&wib);
                let updated_wib = brand.updated_at.with_timezone(&wib);

                println!("✅ Brand ditemukan:");
                println!("   ID          : {}", brand.id);
                println!("   Name        : {}", brand.name);
                println!(
                    "   Description : {}",
                    brand
                        .description
                        .unwrap_or_else(|| "(tidak ada)".to_string())
                );
                println!(
                    "   Created At  : {} WIB",
                    created_wib.format("%Y-%m-%d %H:%M:%S")
                );
                println!(
                    "   Updated At  : {} WIB",
                    updated_wib.format("%Y-%m-%d %H:%M:%S")
                );
            }
            None => println!("⚠️  Brand tidak ditemukan"),
        }

        //2. get all brands
        let all_brands = get_all_brands(&pool).await?;
        println!("✅ Semua data brands:");

        let wib = chrono::FixedOffset::east_opt(7 * 3600).unwrap();

        for brand in all_brands {
            let created_wib = brand.created_at.with_timezone(&wib); // ← with_timezone
            let updated_wib = brand.updated_at.with_timezone(&wib); // ← with_timezone (bukan with_second)

            match brand.description {
                Some(desc) => println!(
                    "   {} | {} | {} | {} WIB | {} WIB",
                    brand.id,
                    brand.name,
                    desc,
                    created_wib.format("%Y-%m-%d %H:%M:%S"),
                    updated_wib.format("%Y-%m-%d %H:%M:%S")
                ),
                None => println!(
                    "   {} | {} | (tidak ada) | {} WIB | {} WIB",
                    brand.id,
                    brand.name,
                    created_wib.format("%Y-%m-%d %H:%M:%S"),
                    updated_wib.format("%Y-%m-%d %H:%M:%S")
                ),
            }
        }

        // Insert beberapa bus
        insert_bus(&pool, "Bus Transjakarta").await?;
        insert_bus(&pool, "Bus Kota").await?;
        insert_bus(&pool, "Bus Pariwisata").await?;

        println!();

        // Ambil semua bus
        let all_buses = get_all_buses(&pool).await?;
        println!("📋 Semua bus:");
        for bus in all_buses {
            println!("   ID: {} | Name: {}", bus.id, bus.name);
        }

        println!();

        // Ambil bus by ID
        match get_bus_by_id(&pool, 2).await? {
            Some(bus) => println!("✅ Bus dengan ID 2: {} - {}", bus.id, bus.name),
            None => println!("⚠️ Bus dengan ID 2 tidak ditemukan"),
        }

        pool.close().await;
        Ok::<(), sqlx::Error>(())
    })
}
