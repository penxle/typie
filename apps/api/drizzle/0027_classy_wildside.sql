CREATE TYPE "public"."_post_layout_mode" AS ENUM('SCROLL', 'PAGE');
DROP TABLE "post_versions" CASCADE;
ALTER TABLE "post_contents" ADD COLUMN "layout_mode" "_post_layout_mode" DEFAULT 'SCROLL' NOT NULL;
ALTER TABLE "post_contents" ADD COLUMN "page_layout" jsonb;