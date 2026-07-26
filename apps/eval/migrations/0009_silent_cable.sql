CREATE TABLE `feedback_verdicts` (
	`id` text PRIMARY KEY NOT NULL,
	`judgment_id` text NOT NULL,
	`feedback_id` text NOT NULL,
	`correct` integer DEFAULT true NOT NULL,
	`needed` integer DEFAULT true NOT NULL,
	`useful` integer DEFAULT true NOT NULL,
	`note` text
);
--> statement-breakpoint
CREATE UNIQUE INDEX `feedback_verdicts_judgment_id_feedback_id` ON `feedback_verdicts` (`judgment_id`,`feedback_id`);--> statement-breakpoint
CREATE TABLE `review_verdicts` (
	`id` text PRIMARY KEY NOT NULL,
	`judgment_id` text NOT NULL,
	`set_id` text NOT NULL,
	`read_correctly` integer DEFAULT true NOT NULL,
	`priority_useful` integer DEFAULT true NOT NULL,
	`note` text
);
--> statement-breakpoint
CREATE UNIQUE INDEX `review_verdicts_judgment_id_set_id` ON `review_verdicts` (`judgment_id`,`set_id`);