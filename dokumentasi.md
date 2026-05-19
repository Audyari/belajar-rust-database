docker compose up -d

docker exec -it belajar-rust-datetime-db-1 psql -U postgres

\l

\c belajar_rust_database

\dt

SELECT \* FROM category;

\d category

\q
