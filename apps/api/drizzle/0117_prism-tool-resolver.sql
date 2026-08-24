CREATE TYPE "public"."_prism_tool_resolver" AS ENUM('USER', 'SERVER');
ALTER TABLE "prism_tool_calls" ADD COLUMN "resolver" "_prism_tool_resolver" DEFAULT 'SERVER' NOT NULL;