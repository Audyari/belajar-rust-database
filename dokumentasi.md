docker compose up -d

// masuk ke container postgres
docker exec -it belajar-rust-datetime-db-1 psql -U postgres

// masuk ke container redis
docker exec -it redis-stasiun redis-cli

\l

\c belajar_rust_database

\dt

SELECT * FROM category;

\d category

\q


1. docker-compose down
2. docker-compose up -d
3. docker-compose ps