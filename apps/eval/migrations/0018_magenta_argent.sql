CREATE TABLE `call_cache` (
	`run_id` text NOT NULL,
	`key` text NOT NULL,
	`value` text NOT NULL,
	`created_at` integer NOT NULL,
	PRIMARY KEY(`run_id`, `key`)
);
--> statement-breakpoint
CREATE TABLE `documents_v2` (
	`id` text PRIMARY KEY NOT NULL,
	`ref_id` text NOT NULL,
	`content` text NOT NULL,
	`character_count` integer NOT NULL,
	`genre` text,
	`sampling_id` text,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `evaluators` (
	`email` text PRIMARY KEY NOT NULL,
	`evaluating` integer DEFAULT false NOT NULL,
	`consented_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `item_anchors` (
	`id` text PRIMARY KEY NOT NULL,
	`item_id` text NOT NULL,
	`ord` integer NOT NULL,
	`start_text` text NOT NULL,
	`end_text` text NOT NULL,
	`match_start` integer,
	`match_end` integer,
	`note` text
);
--> statement-breakpoint
CREATE UNIQUE INDEX `item_anchors_item_id_ord` ON `item_anchors` (`item_id`,`ord`);--> statement-breakpoint
CREATE TABLE `item_links` (
	`item_id` text NOT NULL,
	`target_item_id` text NOT NULL,
	`ord` integer NOT NULL,
	PRIMARY KEY(`item_id`, `ord`)
);
--> statement-breakpoint
CREATE TABLE `judgment_items` (
	`id` text PRIMARY KEY NOT NULL,
	`judgment_id` text NOT NULL,
	`item_id` text NOT NULL,
	`payload` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `judgment_items_judgment_id_item_id` ON `judgment_items` (`judgment_id`,`item_id`);--> statement-breakpoint
CREATE TABLE `judgments_v2` (
	`id` text PRIMARY KEY NOT NULL,
	`task_id` text NOT NULL,
	`evaluator_email` text NOT NULL,
	`draft` integer DEFAULT true NOT NULL,
	`payload` text NOT NULL,
	`elapsed_seconds` integer DEFAULT 0 NOT NULL,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `judgments_v2_task_id_unique` ON `judgments_v2` (`task_id`);--> statement-breakpoint
CREATE TABLE `ledgers` (
	`run_id` text NOT NULL,
	`key` text NOT NULL,
	`value` text NOT NULL,
	`created_at` integer NOT NULL,
	PRIMARY KEY(`run_id`, `key`)
);
--> statement-breakpoint
CREATE TABLE `phase_usage` (
	`run_id` text NOT NULL,
	`phase` text NOT NULL,
	`calls` integer DEFAULT 0 NOT NULL,
	`prompt_tokens` integer DEFAULT 0 NOT NULL,
	`completion_tokens` integer DEFAULT 0 NOT NULL,
	`cached_tokens` integer DEFAULT 0 NOT NULL,
	`cache_write_tokens` integer DEFAULT 0 NOT NULL,
	PRIMARY KEY(`run_id`, `phase`)
);
--> statement-breakpoint
CREATE TABLE `prompt_sets` (
	`id` text PRIMARY KEY NOT NULL,
	`generation_id` text NOT NULL,
	`label` text NOT NULL,
	`note` text,
	`content` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `prompt_sets_label_unique` ON `prompt_sets` (`label`);--> statement-breakpoint
CREATE TABLE `rounds_v2` (
	`id` text PRIMARY KEY NOT NULL,
	`label` text NOT NULL,
	`evaluation_id` text NOT NULL,
	`active` integer DEFAULT false NOT NULL,
	`config` text,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `run_items` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL,
	`kind` text NOT NULL,
	`ord` integer NOT NULL,
	`body` text NOT NULL,
	`facets` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `runs_v2` (
	`id` text PRIMARY KEY NOT NULL,
	`document_id` text NOT NULL,
	`prompt_set_id` text,
	`instance_id` text,
	`status` text NOT NULL,
	`phase` text,
	`error` text,
	`created_at` integer NOT NULL,
	`finished_at` integer
);
--> statement-breakpoint
CREATE TABLE `samplings` (
	`id` text PRIMARY KEY NOT NULL,
	`instance_id` text,
	`status` text NOT NULL,
	`phase` text,
	`size` integer NOT NULL,
	`error` text,
	`created_at` integer NOT NULL,
	`finished_at` integer
);
--> statement-breakpoint
CREATE TABLE `settings_v2` (
	`key` text PRIMARY KEY NOT NULL,
	`value` text NOT NULL
);
--> statement-breakpoint
CREATE TABLE `task_releases` (
	`task_id` text NOT NULL,
	`evaluator_email` text NOT NULL,
	`created_at` integer NOT NULL,
	PRIMARY KEY(`task_id`, `evaluator_email`)
);
--> statement-breakpoint
CREATE TABLE `tasks_v2` (
	`id` text PRIMARY KEY NOT NULL,
	`round_id` text NOT NULL,
	`run_id` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `tasks_v2_round_id_run_id` ON `tasks_v2` (`round_id`,`run_id`);