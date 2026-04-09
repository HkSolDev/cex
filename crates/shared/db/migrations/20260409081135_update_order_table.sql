-- Add migration script here

-- 1. Create the new types with lowercase variants to match Rust 'rename_all'
CREATE TYPE side AS ENUM ('buy', 'sell'); 
CREATE TYPE order_type AS ENUM ('market', 'limit', 'stoplimit'); 
CREATE TYPE order_status AS ENUM ('pending', 'partialfilled', 'filled', 'cancelled');

-- 2. Update the orders table
-- Add missing columns
ALTER TABLE orders ADD COLUMN order_type order_type NOT NULL DEFAULT 'limit';
ALTER TABLE orders ADD COLUMN filled_qty BIGINT NOT NULL DEFAULT 0;
ALTER TABLE orders ADD COLUMN "timestamp" BIGINT NOT NULL DEFAULT 0;

-- Rename quantity to qty to match Rust code
ALTER TABLE orders RENAME COLUMN quantity TO qty;

-- Convert the side column from VARCHAR to the new ENUM type
ALTER TABLE orders ALTER COLUMN side TYPE side USING (LOWER(side)::side);

-- Convert the status column from VARCHAR to the new ENUM type
ALTER TABLE orders ALTER COLUMN status TYPE order_status USING (LOWER(status)::order_status);
