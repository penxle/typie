CREATE TYPE "public"."_prism_credit_entry_kind" AS ENUM('GRANT', 'TRIAL', 'REVIEW_CHARGE', 'REVIEW_REFUND', 'CHAT_CHARGE', 'ADJUSTMENT');
CREATE TABLE "prism_credit_entries" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"kind" "_prism_credit_entry_kind" NOT NULL,
	"paid_delta" bigint NOT NULL,
	"free_delta" bigint NOT NULL,
	"key" text,
	"note" text,
	"actor_id" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_credit_entries_kind_key_unique" UNIQUE("kind","key")
);

ALTER TABLE "prism_credit_entries" ADD CONSTRAINT "prism_credit_entries_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_credit_entries" ADD CONSTRAINT "prism_credit_entries_actor_id_users_id_fk" FOREIGN KEY ("actor_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "prism_credit_entries_user_id_index" ON "prism_credit_entries" USING btree ("user_id");