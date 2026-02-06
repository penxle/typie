CREATE TYPE "public"."_text_replacement_state" AS ENUM('ACTIVE', 'DISABLED');
CREATE TABLE "text_replacement_preferences" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"text_replacement_id" text NOT NULL,
	"state" "_text_replacement_state" NOT NULL,
	"order" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "text_replacement_preferences_user_id_text_replacement_id_unique" UNIQUE("user_id","text_replacement_id"),
	CONSTRAINT "text_replacement_preferences_user_id_order_unique" UNIQUE("user_id","order")
);

CREATE TABLE "text_replacements" (
	"id" text PRIMARY KEY NOT NULL,
	"match" text NOT NULL,
	"substitute" text NOT NULL,
	"regex" boolean DEFAULT false NOT NULL,
	"preset" boolean DEFAULT false NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "text_replacement_preferences" ADD CONSTRAINT "text_replacement_preferences_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "text_replacement_preferences" ADD CONSTRAINT "text_replacement_preferences_text_replacement_id_text_replacements_id_fk" FOREIGN KEY ("text_replacement_id") REFERENCES "public"."text_replacements"("id") ON DELETE cascade ON UPDATE cascade;
CREATE INDEX "text_replacement_preferences_user_id_index" ON "text_replacement_preferences" USING btree ("user_id");