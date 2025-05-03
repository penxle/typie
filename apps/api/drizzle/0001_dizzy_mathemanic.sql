CREATE TYPE "public"."_font_state" AS ENUM('ACTIVE', 'ARCHIVED');
CREATE TABLE "fonts" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text,
	"site_id" text,
	"name" text NOT NULL,
	"family_name" text,
	"full_name" text,
	"post_script_name" text,
	"weight" integer NOT NULL,
	"size" integer NOT NULL,
	"path" text NOT NULL,
	"state" "_font_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "fonts" ADD CONSTRAINT "fonts_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "fonts" ADD CONSTRAINT "fonts_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;