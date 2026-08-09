ALTER TABLE `reviews` RENAME COLUMN "prism_session_id" TO "prism_workflow_id";--> statement-breakpoint
DROP INDEX `reviews_prism_session_id_unique`;--> statement-breakpoint
CREATE UNIQUE INDEX `reviews_prism_workflow_id_unique` ON `reviews` (`prism_workflow_id`);