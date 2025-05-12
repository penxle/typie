CREATE TABLE "user_push_notification_tokens" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"token" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_push_notification_tokens_token_unique" UNIQUE("token")
);

ALTER TABLE "user_push_notification_tokens" ADD CONSTRAINT "user_push_notification_tokens_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;