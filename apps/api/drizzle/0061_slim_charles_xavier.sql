CREATE TYPE "public"."_site_date_display" AS ENUM('NONE', 'CREATED_AT', 'UPDATED_AT');
ALTER TABLE "sites" ADD COLUMN "date_display" "_site_date_display" DEFAULT 'UPDATED_AT' NOT NULL;