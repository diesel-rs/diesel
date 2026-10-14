CREATE TYPE "2fa_method" AS ENUM ('sms', '2fa_app');

CREATE TABLE "24h_orders" (
    id SERIAL PRIMARY KEY,
    "1st_choice" "2fa_method" NOT NULL
);

CREATE TABLE "7day_returns" (
    id SERIAL PRIMARY KEY,
    "24h_order_id" INTEGER NOT NULL REFERENCES "24h_orders" (id)
);
