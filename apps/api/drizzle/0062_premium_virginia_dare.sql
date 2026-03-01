CREATE TYPE "public"."_redirect_type" AS ENUM('SLUG', 'PERMALINK');
CREATE TABLE "redirects" (
	"id" text PRIMARY KEY NOT NULL,
	"site_id" text NOT NULL,
	"type" "_redirect_type" NOT NULL,
	"from" text NOT NULL,
	"to" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "redirects" ADD CONSTRAINT "redirects_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "redirects_site_id_type_from_index" ON "redirects" USING btree ("site_id","type","from");
