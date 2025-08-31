CREATE TABLE "font_families" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text,
	"site_id" text,
	"name" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "font_families_site_id_name_unique" UNIQUE("site_id","name")
);

ALTER TABLE "fonts" ADD COLUMN "family_id" text;
ALTER TABLE "font_families" ADD CONSTRAINT "font_families_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "font_families" ADD CONSTRAINT "font_families_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "font_families_site_id_index" ON "font_families" USING btree ("site_id");
ALTER TABLE "fonts" ADD CONSTRAINT "fonts_family_id_font_families_id_fk" FOREIGN KEY ("family_id") REFERENCES "public"."font_families"("id") ON DELETE restrict ON UPDATE cascade;