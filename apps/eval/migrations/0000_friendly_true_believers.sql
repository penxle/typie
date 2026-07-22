CREATE TABLE `documents` (
	`id` text PRIMARY KEY NOT NULL,
	`ref_id` text NOT NULL,
	`content` text NOT NULL,
	`character_count` integer NOT NULL,
	`corpus_version` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `feedback_sets` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL,
	`document_id` text NOT NULL,
	`variant_id` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `feedback_sets_run_id_document_id` ON `feedback_sets` (`run_id`,`document_id`);--> statement-breakpoint
CREATE TABLE `feedbacks` (
	`id` text PRIMARY KEY NOT NULL,
	`set_id` text NOT NULL,
	`ord` integer NOT NULL,
	`start_text` text NOT NULL,
	`end_text` text NOT NULL,
	`match_start` integer,
	`match_end` integer,
	`category` text,
	`body` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `judgments` (
	`id` text PRIMARY KEY NOT NULL,
	`task_id` text NOT NULL,
	`evaluator_email` text NOT NULL,
	`result` text,
	`false_positive_feedback_ids` text DEFAULT '[]' NOT NULL,
	`comment` text,
	`draft` integer DEFAULT true NOT NULL,
	`elapsed_seconds` integer,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `judgments_task_id_evaluator_email` ON `judgments` (`task_id`,`evaluator_email`);--> statement-breakpoint
CREATE TABLE `rounds` (
	`id` text PRIMARY KEY NOT NULL,
	`stage` text NOT NULL,
	`config` text,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `runs` (
	`id` text PRIMARY KEY NOT NULL,
	`variant_id` text NOT NULL,
	`corpus_version` text NOT NULL,
	`meta` text,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `settings` (
	`key` text PRIMARY KEY NOT NULL,
	`value` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `tasks` (
	`id` text PRIMARY KEY NOT NULL,
	`round_id` text NOT NULL,
	`kind` text NOT NULL,
	`document_id` text NOT NULL,
	`set_ids` text NOT NULL,
	`required_judgments` integer,
	`golden` integer DEFAULT false NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `variants` (
	`id` text PRIMARY KEY NOT NULL,
	`label` text NOT NULL,
	`round` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `variants_label_unique` ON `variants` (`label`);