ALTER TYPE "public"."_payment_invoice_state" ADD VALUE 'WAIVED';

ALTER TABLE "subscriptions" ADD COLUMN "renewed_at" timestamp with time zone;

UPDATE "subscriptions" s
SET "renewed_at" = CASE
  WHEN p."interval" = 'MONTHLY' THEN s."expires_at" - INTERVAL '1 month'
  WHEN p."interval" = 'YEARLY' THEN s."expires_at" - INTERVAL '1 year'
  WHEN p."interval" = 'TRIAL' THEN s."starts_at"
  WHEN p."interval" = 'LIFETIME' THEN s."starts_at"
  ELSE s."starts_at"
END
FROM "plans" p
WHERE s."plan_id" = p."id";

ALTER TABLE "subscriptions" ALTER COLUMN "renewed_at" SET NOT NULL;
