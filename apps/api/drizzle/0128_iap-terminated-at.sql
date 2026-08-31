ALTER TABLE "user_in_app_purchases" ADD COLUMN "terminated_at" timestamp with time zone;
UPDATE "user_in_app_purchases" SET "terminated_at" = "reconcile_suspended_at" WHERE "reconcile_suspended_at" IS NOT NULL;
ALTER TABLE "user_in_app_purchases" DROP CONSTRAINT "user_in_app_purchases_user_id_unique";
