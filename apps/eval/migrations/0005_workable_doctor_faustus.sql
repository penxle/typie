CREATE TABLE `released_tasks` (
	`task_id` text NOT NULL,
	`evaluator_email` text NOT NULL,
	`created_at` integer NOT NULL,
	PRIMARY KEY(`task_id`, `evaluator_email`)
);
