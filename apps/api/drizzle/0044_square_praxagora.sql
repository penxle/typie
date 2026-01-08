ALTER TABLE "sites" ADD COLUMN "logo_id" text;

UPDATE "sites" SET "logo_id" = "users"."avatar_id" FROM "users" WHERE "sites"."user_id" = "users"."id";

ALTER TABLE "sites" ALTER COLUMN "logo_id" SET NOT NULL;

ALTER TABLE "sites" ADD CONSTRAINT "sites_logo_id_images_id_fk" FOREIGN KEY ("logo_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;
