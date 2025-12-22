DROP TABLE "canvas_contents" CASCADE;
DROP TABLE "canvas_snapshot_contributors" CASCADE;
DROP TABLE "canvas_snapshots" CASCADE;
DROP TABLE "canvases" CASCADE;
ALTER TABLE "entities" ALTER COLUMN "type" SET DATA TYPE text;
DROP TYPE "public"."_entity_type";
CREATE TYPE "public"."_entity_type" AS ENUM('DOCUMENT', 'FOLDER', 'POST');
ALTER TABLE "entities" ALTER COLUMN "type" SET DATA TYPE "public"."_entity_type" USING "type"::"public"."_entity_type";