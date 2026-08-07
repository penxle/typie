CREATE TYPE "public"."_document_head_kind" AS ENUM('NORMAL', 'ISOLATED');
ALTER TABLE "document_head_contributors" ADD COLUMN "additions" integer;
ALTER TABLE "document_head_contributors" ADD COLUMN "deletions" integer;
ALTER TABLE "document_head_contributors" ADD COLUMN "excluded" boolean DEFAULT false NOT NULL;
ALTER TABLE "document_heads" ADD COLUMN "kind" "_document_head_kind" DEFAULT 'NORMAL' NOT NULL;
ALTER TABLE "document_heads" ADD COLUMN "seq" integer;
CREATE INDEX "document_head_contributors_user_id_index" ON "document_head_contributors" USING btree ("user_id") WHERE excluded;

UPDATE "document_heads" h
SET "seq" = s.rn
FROM (
  SELECT id, row_number() OVER (PARTITION BY document_id ORDER BY bucket) AS rn
  FROM "document_heads"
) s
WHERE h.id = s.id;