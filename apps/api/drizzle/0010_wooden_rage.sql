CREATE TYPE "public"."_user_role" AS ENUM('ADMIN', 'USER');
ALTER TABLE "users" ADD COLUMN "role" "_user_role" DEFAULT 'USER' NOT NULL;