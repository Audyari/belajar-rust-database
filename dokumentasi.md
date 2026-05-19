docker compose up -d Start database (background)
docker compose down Stop database (data tetap ada)
docker compose down -v Stop dan hapus semua data

docker compose logs -f db Lihat log database

docker exec -it belajar-rust-datetime-db-1 psql -U postgres
