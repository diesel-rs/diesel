ALTER TABLE users ADD COLUMN user_class user_type NOT NULL
  DEFAULT CAST('default' AS user_type);
-- ALTER TABLE custom_schema.users ADD COLUMN admin_class custom_schema.admin_type NOT NULL
--   DEFAULT CAST('super_duper_user' AS custom_schema.admin_type);
