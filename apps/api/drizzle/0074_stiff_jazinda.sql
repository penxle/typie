ALTER TABLE "fonts" ADD COLUMN "hash" text DEFAULT '' NOT NULL;
ALTER TABLE "fonts" ADD COLUMN "subsets" jsonb DEFAULT '[]'::jsonb NOT NULL;