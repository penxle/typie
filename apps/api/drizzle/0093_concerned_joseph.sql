CREATE TYPE "public"."_billing_key_type" AS ENUM('CARD', 'KAKAOPAY');
ALTER TABLE "user_billing_keys" ADD COLUMN "type" "_billing_key_type" DEFAULT 'CARD' NOT NULL;