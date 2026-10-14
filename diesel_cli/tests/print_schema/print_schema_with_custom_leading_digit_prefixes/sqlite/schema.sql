CREATE TABLE "24h_orders" (
    id INTEGER PRIMARY KEY,
    "1st_choice" TEXT NOT NULL
);

CREATE TABLE "7day_returns" (
    id INTEGER PRIMARY KEY,
    "24h_order_id" INTEGER NOT NULL REFERENCES "24h_orders" (id)
);
