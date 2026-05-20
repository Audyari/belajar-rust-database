-- Add up migration script here
-- Buat tabel drivers (sopir)
CREATE TABLE IF NOT EXISTS drivers (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    bus_id INTEGER REFERENCES buses(id) ON DELETE SET NULL,
    license_number VARCHAR(50) UNIQUE NOT NULL,
    hire_date DATE DEFAULT CURRENT_DATE,
    is_active BOOLEAN DEFAULT TRUE
);

-- Buat index untuk performance
CREATE INDEX IF NOT EXISTS idx_drivers_bus_id ON drivers(bus_id);
CREATE INDEX IF NOT EXISTS idx_drivers_license ON drivers(license_number);

COMMENT ON TABLE drivers IS 'Data sopir bus';