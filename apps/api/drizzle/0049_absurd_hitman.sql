ALTER TYPE "public"."_plan_availability" ADD VALUE 'TRIAL' BEFORE 'MANUAL';
ALTER TYPE "public"."_plan_interval" ADD VALUE 'TRIAL' BEFORE 'LIFETIME';
CREATE TABLE "user_trials" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"subscription_id" text NOT NULL,
	"started_at" timestamp with time zone NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_trials_user_id_unique" UNIQUE("user_id")
);

ALTER TABLE "user_trials" ADD CONSTRAINT "user_trials_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_trials" ADD CONSTRAINT "user_trials_subscription_id_subscriptions_id_fk" FOREIGN KEY ("subscription_id") REFERENCES "public"."subscriptions"("id") ON DELETE restrict ON UPDATE cascade;