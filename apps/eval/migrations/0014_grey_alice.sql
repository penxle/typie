PRAGMA foreign_keys=OFF;--> statement-breakpoint
CREATE TABLE `__new_evaluator_consents` (
	`email` text PRIMARY KEY NOT NULL,
	`evaluating` integer DEFAULT false NOT NULL,
	`created_at` integer NOT NULL
);
--> statement-breakpoint
INSERT INTO `__new_evaluator_consents`("email", "evaluating", "created_at") SELECT "email", coalesce("evaluating", EXISTS (SELECT 1 FROM `judgments` WHERE `judgments`.`evaluator_email` = `evaluator_consents`.`email`)), "created_at" FROM `evaluator_consents`;--> statement-breakpoint
DROP TABLE `evaluator_consents`;--> statement-breakpoint
ALTER TABLE `__new_evaluator_consents` RENAME TO `evaluator_consents`;--> statement-breakpoint
PRAGMA foreign_keys=ON;