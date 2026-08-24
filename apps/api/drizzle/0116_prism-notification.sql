ALTER TABLE "prism_sessions" ADD COLUMN "awaiting_user_at" timestamp with time zone;
ALTER TABLE "prism_sessions" ADD COLUMN "seen_at" timestamp with time zone;
ALTER TABLE "prism_workflows" ADD COLUMN "awaiting_user_at" timestamp with time zone;