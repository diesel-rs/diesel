CREATE TABLE `24h_orders` (
    id INTEGER PRIMARY KEY,
    delivery ENUM('standard', '24h', 'self', 'mod') NOT NULL
);
