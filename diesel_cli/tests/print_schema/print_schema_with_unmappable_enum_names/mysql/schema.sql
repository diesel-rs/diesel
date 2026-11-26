CREATE TABLE `24h_orders` (
    id INTEGER PRIMARY KEY,
    `1st_choice` INTEGER NOT NULL,
    delivery ENUM('standard', '24h', 'self', 'mod') NOT NULL
);
