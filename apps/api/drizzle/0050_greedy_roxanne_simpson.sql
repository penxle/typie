ALTER TABLE "user_marketing_consents" ADD COLUMN "consented" boolean;
ALTER TABLE "user_marketing_consents" ADD COLUMN "asked_at" timestamp with time zone;

UPDATE "user_marketing_consents" SET "consented" = true, "asked_at" = "created_at";

ALTER TABLE "user_marketing_consents" ALTER COLUMN "consented" SET NOT NULL;
ALTER TABLE "user_marketing_consents" ALTER COLUMN "asked_at" SET NOT NULL;
ALTER TABLE "user_marketing_consents" ALTER COLUMN "asked_at" SET DEFAULT now();
