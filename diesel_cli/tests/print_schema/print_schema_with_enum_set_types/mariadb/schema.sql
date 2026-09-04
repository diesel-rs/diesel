CREATE TABLE `users1` (
  `id` int(11) NOT NULL PRIMARY KEY,
  `user_state` enum('active','disabled') NOT NULL DEFAULT 'active',
  `enabled_features` set('val1','val2','val3','val4') NOT NULL DEFAULT 'val1'
);