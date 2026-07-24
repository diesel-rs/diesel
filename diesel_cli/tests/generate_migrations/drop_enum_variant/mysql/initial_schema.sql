CREATE TABLE `resource`(
	`resource_id` INT4 NOT NULL PRIMARY KEY,
	`some_field` enum('a', 'b', 'c') NOT NULL,
	`some_field2` enum('FOOBAR', 'BAZBOOM') NOT NULL
);
