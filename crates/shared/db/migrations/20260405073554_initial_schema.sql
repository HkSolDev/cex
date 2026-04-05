-- Add migration script here
-- Creates the users table
CREATE TABLE users (
    id BIGINT PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Creates the orders table linking back to the users
CREATE TABLE orders (
    id BIGINT PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id),
    symbol VARCHAR(16) NOT NULL,
    side VARCHAR(4) NOT NULL,
    price BIGINT NOT NULL,          -- Integer cents!
    quantity BIGINT NOT NULL,
    status VARCHAR(16) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE trades (
    id BIGINT PRIMARY KEY,
    make_order_id BIGINT NOT NULL REFERENCES orders(id),
    take_order_id BIGINT NOT NULL REFERENCES orders(id),
    symbol VARCHAR(16) NOT NULL,
    price BIGINT NOT NULL,
    quantity BIGINT NOT NULL,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- User Multi assests bank account
CREATE TABLE balances (
user_id BIGINT NOT NULL REFERENCES users(id),
asset VARCHAR(16) NOT NULL, 
free BIGINT NOT NULL DEFAULT 0, -- The fee balance you cna do anything
locked BIGINT NOT NULL DEFAULT 0,
PRIMARY KEY (user_id, asset)
);

-- Immutable record of every single money movement
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id),
    action VARCHAR(32) NOT NULL, -- "DEPOSIT", "WITHDRAW",  "TRADE FILL"
    amount BIGINT NOT NULL,
    asset VARCHAR(16) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);