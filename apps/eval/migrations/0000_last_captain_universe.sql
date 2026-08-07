CREATE TABLE `feedback_sessions` (
	`id` text PRIMARY KEY NOT NULL,
	`ref_id` text NOT NULL,
	`title` text,
	`tester_email` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `manuscript_versions` (
	`session_id` text NOT NULL,
	`version` integer NOT NULL,
	`content` text NOT NULL,
	`char_count` integer NOT NULL,
	`imported_at` integer NOT NULL,
	PRIMARY KEY(`session_id`, `version`)
);
--> statement-breakpoint
CREATE TABLE `reactions` (
	`session_id` text NOT NULL,
	`review_round` integer NOT NULL,
	`value` text NOT NULL,
	`note` text,
	`created_at` integer NOT NULL,
	PRIMARY KEY(`session_id`, `review_round`)
);
--> statement-breakpoint
CREATE TABLE `reviews` (
	`session_id` text NOT NULL,
	`round` integer NOT NULL,
	`prism_session_id` text NOT NULL,
	`status` text NOT NULL,
	`manuscript_version` integer NOT NULL,
	`started_at` integer NOT NULL,
	`finished_at` integer,
	`usage` text,
	`result` text,
	`error` text,
	`events` text,
	PRIMARY KEY(`session_id`, `round`)
);
--> statement-breakpoint
CREATE UNIQUE INDEX `reviews_prism_session_id_unique` ON `reviews` (`prism_session_id`);--> statement-breakpoint
CREATE TABLE `thread_comments` (
	`id` text PRIMARY KEY NOT NULL,
	`thread_id` text NOT NULL,
	`author` text NOT NULL,
	`body` text NOT NULL,
	`review_round` integer,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `threads` (
	`id` text PRIMARY KEY NOT NULL,
	`session_id` text NOT NULL,
	`review_round` integer NOT NULL,
	`issue_index` integer NOT NULL,
	`axis` text NOT NULL,
	`pass` text NOT NULL,
	`body` text,
	`anchors` text NOT NULL,
	`state` text DEFAULT 'open' NOT NULL,
	`state_changed_at` integer
);
