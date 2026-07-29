CREATE TABLE `analysis_stage_usage` (
	`run_id` text NOT NULL,
	`document_id` text NOT NULL,
	`stage` text NOT NULL,
	`calls` integer DEFAULT 0 NOT NULL,
	`prompt_tokens` integer DEFAULT 0 NOT NULL,
	`completion_tokens` integer DEFAULT 0 NOT NULL,
	`cached_tokens` integer DEFAULT 0 NOT NULL,
	PRIMARY KEY(`run_id`, `document_id`, `stage`)
);
