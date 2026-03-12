DROP TABLE IF EXISTS "_migration_note_to_issue";
ALTER TABLE "issue_entities" DISABLE ROW LEVEL SECURITY;
ALTER TABLE "issues" DISABLE ROW LEVEL SECURITY;
DROP TABLE "issue_entities" CASCADE;
DROP TABLE "issues" CASCADE;
ALTER TABLE "notes" DROP CONSTRAINT "notes_entity_id_entities_id_fk";

DROP INDEX "notes_entity_id_state_order_index";
ALTER TABLE "notes" ALTER COLUMN "site_id" SET NOT NULL;
ALTER TABLE "notes" DROP COLUMN "entity_id";
DROP TYPE "public"."_issue_priority";
DROP TYPE "public"."_issue_state";
DROP TYPE "public"."_issue_status";