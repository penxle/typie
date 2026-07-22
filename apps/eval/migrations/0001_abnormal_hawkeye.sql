CREATE TABLE `pipeline_run_docs` (
	`id` text PRIMARY KEY NOT NULL,
	`run_id` text NOT NULL,
	`document_id` text NOT NULL,
	`workflow_instance_id` text,
	`status` text NOT NULL,
	`done_chunks` integer DEFAULT 0 NOT NULL,
	`total_chunks` integer DEFAULT 0 NOT NULL,
	`error` text
);
--> statement-breakpoint
CREATE UNIQUE INDEX `pipeline_run_docs_run_id_document_id` ON `pipeline_run_docs` (`run_id`,`document_id`);--> statement-breakpoint
CREATE TABLE `pipeline_runs` (
	`id` text PRIMARY KEY NOT NULL,
	`kind` text NOT NULL,
	`variant_id` text,
	`corpus_version` text NOT NULL,
	`status` text NOT NULL,
	`phase` text,
	`done_chunks` integer DEFAULT 0 NOT NULL,
	`total_chunks` integer DEFAULT 0 NOT NULL,
	`done_docs` integer DEFAULT 0 NOT NULL,
	`total_docs` integer DEFAULT 0 NOT NULL,
	`prompt_tokens` integer DEFAULT 0 NOT NULL,
	`completion_tokens` integer DEFAULT 0 NOT NULL,
	`error` text,
	`created_at` integer NOT NULL,
	`finished_at` integer
);
--> statement-breakpoint
CREATE TABLE `prompt_applies` (
	`id` text PRIMARY KEY NOT NULL,
	`prompt_id` text NOT NULL,
	`prev` text NOT NULL,
	`applied_variant_id` text NOT NULL,
	`applied_stage` text NOT NULL,
	`applied_by` text NOT NULL,
	`status` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE TABLE `prompt_variants` (
	`id` text PRIMARY KEY NOT NULL,
	`label` text NOT NULL,
	`note` text,
	`base_variant_id` text,
	`content` text NOT NULL,
	`status` text DEFAULT 'draft' NOT NULL,
	`created_at` integer NOT NULL,
	`updated_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `prompt_variants_label_unique` ON `prompt_variants` (`label`);--> statement-breakpoint
CREATE TABLE `stage_cache` (
	`key` text PRIMARY KEY NOT NULL,
	`value` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
ALTER TABLE `variants` ADD `prompt_variant_id` text;