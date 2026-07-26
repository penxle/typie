CREATE TABLE `analysis_prompt_sets` (
	`id` text PRIMARY KEY NOT NULL,
	`label` text NOT NULL,
	`note` text,
	`content` text NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `analysis_prompt_sets_label_unique` ON `analysis_prompt_sets` (`label`);--> statement-breakpoint
ALTER TABLE `feedback_sets` ADD `review` text;--> statement-breakpoint
ALTER TABLE `pipeline_run_docs` ADD `phase` text;