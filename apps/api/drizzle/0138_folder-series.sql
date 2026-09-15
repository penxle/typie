ALTER TABLE "folders" ADD COLUMN "description" text;
ALTER TABLE "folders" ADD COLUMN "pinned" boolean DEFAULT false NOT NULL;