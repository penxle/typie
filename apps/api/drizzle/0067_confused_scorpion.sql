CREATE TABLE "font_names" (
	"id" text PRIMARY KEY NOT NULL,
	"font_id" text NOT NULL,
	"name_id" integer NOT NULL,
	"platform_id" integer NOT NULL,
	"language_id" integer NOT NULL,
	"value" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "font_names" ADD CONSTRAINT "font_names_font_id_fonts_id_fk" FOREIGN KEY ("font_id") REFERENCES "public"."fonts"("id") ON DELETE cascade ON UPDATE cascade;
CREATE INDEX "font_names_font_id_index" ON "font_names" USING btree ("font_id");