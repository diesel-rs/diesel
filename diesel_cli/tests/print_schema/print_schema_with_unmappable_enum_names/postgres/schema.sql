CREATE TYPE delivery_option AS ENUM ('standard', '24h', 'self', 'mod');
CREATE TYPE "2fa_method" AS ENUM ('sms', 'totp');
CREATE TYPE "self" AS ENUM ('pickup', 'locker');

CREATE TABLE "24h_orders" (
    id SERIAL PRIMARY KEY,
    "1st_choice" INTEGER NOT NULL,
    delivery delivery_option NOT NULL,
    second_factor "2fa_method" NOT NULL,
    collection "self" NOT NULL
);
