CREATE TABLE "user_revenues" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"amount" integer NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_revenues_user_id_unique" UNIQUE("user_id")
);

ALTER TABLE "user_revenues" ADD CONSTRAINT "user_revenues_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;