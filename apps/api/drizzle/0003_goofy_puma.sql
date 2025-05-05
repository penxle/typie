CREATE TYPE "public"."_post_type" AS ENUM('NORMAL', 'TEMPLATE');
ALTER TABLE "posts" ADD COLUMN "type" "_post_type" DEFAULT 'NORMAL' NOT NULL;