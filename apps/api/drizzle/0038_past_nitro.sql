ALTER TABLE "document_contents" ADD COLUMN "json" jsonb DEFAULT '{}' NOT NULL;
ALTER TABLE "document_contents" ADD COLUMN "text" text DEFAULT '' NOT NULL;
ALTER TABLE "document_contents" ADD COLUMN "character_count" integer DEFAULT 0 NOT NULL;
ALTER TABLE "document_contents" ADD COLUMN "blob_size" bigint DEFAULT 0 NOT NULL;

ALTER TABLE "document_contents" ALTER COLUMN "json" DROP DEFAULT;
ALTER TABLE "document_contents" ALTER COLUMN "text" DROP DEFAULT;
