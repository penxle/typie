PRAGMA foreign_keys=OFF;--> statement-breakpoint
CREATE TABLE `__new_feedback_verdicts` (
	`id` text PRIMARY KEY NOT NULL,
	`judgment_id` text NOT NULL,
	`feedback_id` text NOT NULL,
	`correct` integer,
	`needed` integer,
	`useful` integer,
	`note` text
);
--> statement-breakpoint
INSERT INTO `__new_feedback_verdicts`("id", "judgment_id", "feedback_id", "correct", "needed", "useful", "note") SELECT "id", "judgment_id", "feedback_id", "correct", "needed", "useful", "note" FROM `feedback_verdicts`;--> statement-breakpoint
DROP TABLE `feedback_verdicts`;--> statement-breakpoint
ALTER TABLE `__new_feedback_verdicts` RENAME TO `feedback_verdicts`;--> statement-breakpoint
PRAGMA foreign_keys=ON;--> statement-breakpoint
CREATE UNIQUE INDEX `feedback_verdicts_judgment_id_feedback_id` ON `feedback_verdicts` (`judgment_id`,`feedback_id`);--> statement-breakpoint
CREATE TABLE `__new_review_verdicts` (
	`id` text PRIMARY KEY NOT NULL,
	`judgment_id` text NOT NULL,
	`set_id` text NOT NULL,
	`read_correctly` integer,
	`priority_useful` integer,
	`note` text
);
--> statement-breakpoint
INSERT INTO `__new_review_verdicts`("id", "judgment_id", "set_id", "read_correctly", "priority_useful", "note") SELECT "id", "judgment_id", "set_id", "read_correctly", "priority_useful", "note" FROM `review_verdicts`;--> statement-breakpoint
DROP TABLE `review_verdicts`;--> statement-breakpoint
ALTER TABLE `__new_review_verdicts` RENAME TO `review_verdicts`;--> statement-breakpoint
CREATE UNIQUE INDEX `review_verdicts_judgment_id_set_id` ON `review_verdicts` (`judgment_id`,`set_id`);