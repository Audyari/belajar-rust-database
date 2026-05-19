use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

struct Category {
    id: String,
    name: String,
    description: String,
}

async fn get_pool() -> Result<PgPool, sqlx::Error> {
    let url = "postgresql://postgres:123456@localhost:5432/belajar_rust_database";

    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(url)
        .await
}

fn print_current_time() {
    let now = chrono::Local::now();
    println!("Waktu: {}", now.format("%Y-%m-%d %H:%M:%S"));
}

async fn insert_category(
    pool: &PgPool,
    id: &str,
    name: &str,
    description: Option<String>,
) -> Result<(), sqlx::Error> {
    // Cek apakah ID sudah ada
    let exists: (bool,) = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM category WHERE id = $1)")
        .bind(id)
        .fetch_one(pool)
        .await?;

    if exists.0 {
        println!("⚠️  Category dengan ID '{id}' sudah ADA, tidak jadi insert");
        println!("   Data yang sudah ada: ID={id}, Name={name}");
        return Ok(()); // Tidak error, hanya skip
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

async fn get_category(pool: &PgPool, id: &str) -> Result<Option<Category>, sqlx::Error> {
    // Ganti query_as! (macro) → query_as (function)
    let row = sqlx::query("SELECT id, name, description FROM category WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| Category {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
    }))
}

async fn get_all_category(pool: &PgPool) -> Result<Vec<Category>, sqlx::Error> {
    let rows = sqlx::query("SELECT id, name, description FROM category")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .iter()
        .map(|row| Category {
            id: row.get("id"),
            name: row.get("name"),
            description: row.get("description"),
        })
        .collect())
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

        // ambil data yang ada
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

        pool.close().await;
        Ok::<(), sqlx::Error>(())
    })
}
