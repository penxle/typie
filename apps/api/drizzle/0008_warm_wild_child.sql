BEGIN;

INSERT INTO "plans" ("id", "name", "rules", "fee", "availability", "created_at")
VALUES 
    ('PL0FULL1MONTH', '타이피 FULL ACCESS (월간)', '{"maxTotalCharacterCount": -1, "maxTotalBlobSize": -1}', 4900, 'PUBLIC', NOW()),
    ('PL0FULL1YEAR', '타이피 FULL ACCESS (연간)', '{"maxTotalCharacterCount": -1, "maxTotalBlobSize": -1}', 49000, 'PUBLIC', NOW());

CREATE TYPE "public"."_in_app_purchase_store" AS ENUM('APP_STORE', 'GOOGLE_PLAY');
CREATE TYPE "public"."_payment_outcome" AS ENUM('SUCCESS', 'FAILURE');
CREATE TYPE "public"."_plan_interval" AS ENUM('MONTHLY', 'YEARLY', 'LIFETIME');
CREATE TYPE "public"."_subscription_state" AS ENUM('ACTIVE', 'WILL_ACTIVATE', 'WILL_EXPIRE', 'IN_GRACE_PERIOD', 'EXPIRED');

CREATE TABLE "subscriptions" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"plan_id" text NOT NULL,
	"starts_at" timestamp with time zone NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"state" "_subscription_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "user_billing_keys" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"name" text NOT NULL,
	"billing_key" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_billing_keys_user_id_unique" UNIQUE("user_id"),
	CONSTRAINT "user_billing_keys_billing_key_unique" UNIQUE("billing_key")
);

CREATE TABLE "user_in_app_purchases" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"store" "_in_app_purchase_store" NOT NULL,
	"identifier" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_in_app_purchases_store_identifier_unique" UNIQUE("store","identifier")
);

INSERT INTO "user_billing_keys" ("id", "user_id", "name", "billing_key", "created_at")
SELECT 
	"id",
	"user_id",
	"name",
	"billing_key",
	"created_at"
FROM "payment_billing_keys"
WHERE "state" = 'ACTIVE';

INSERT INTO "subscriptions" ("id", "user_id", "plan_id", "starts_at", "expires_at", "state", "created_at")
SELECT 
	'SUB0' || substr("id", 5) as "id",
	"user_id",
	CASE 
		WHEN "billing_cycle" = 'MONTHLY' THEN 'PL0FULL1MONTH'
		WHEN "billing_cycle" = 'YEARLY' THEN 'PL0FULL1YEAR'
	END as "plan_id",
	"created_at" as "starts_at",
	"expires_at",
	CASE 
		WHEN "state" = 'ACTIVE' THEN 'ACTIVE'::_subscription_state
		WHEN "state" = 'CANCELED' THEN 'WILL_EXPIRE'::_subscription_state
	END as "state",
	"created_at"
FROM "user_plans";

ALTER TABLE "payment_billing_keys" DISABLE ROW LEVEL SECURITY;
ALTER TABLE "user_plans" DISABLE ROW LEVEL SECURITY;

ALTER TABLE "payment_invoices" ALTER COLUMN "state" SET DATA TYPE text;
DROP TYPE "public"."_payment_invoice_state";
CREATE TYPE "public"."_payment_invoice_state" AS ENUM('UPCOMING', 'PAID', 'OVERDUE', 'CANCELED');
ALTER TABLE "payment_invoices" ALTER COLUMN "state" SET DATA TYPE "public"."_payment_invoice_state" USING 
    CASE 
        WHEN "state" = 'UNPAID' THEN 'OVERDUE'::_payment_invoice_state
        WHEN "state" = 'PAID' THEN 'PAID'::_payment_invoice_state
        WHEN "state" = 'CANCELED' THEN 'CANCELED'::_payment_invoice_state
        WHEN "state" = 'UPCOMING' THEN 'UPCOMING'::_payment_invoice_state
    END;

ALTER TABLE "plans" ALTER COLUMN "availability" DROP DEFAULT;
ALTER TABLE "plans" ALTER COLUMN "availability" SET DATA TYPE text;
DROP TYPE "public"."_plan_availability";
CREATE TYPE "public"."_plan_availability" AS ENUM('BILLING_KEY', 'IN_APP_PURCHASE', 'MANUAL');
ALTER TABLE "plans" ALTER COLUMN "availability" SET DATA TYPE "public"."_plan_availability" USING 
    CASE 
        WHEN "availability" = 'PUBLIC' THEN 'BILLING_KEY'::_plan_availability
        WHEN "availability" = 'PRIVATE' THEN 'MANUAL'::_plan_availability
        ELSE 'MANUAL'::_plan_availability
    END;

ALTER TABLE "payment_invoices" ADD COLUMN IF NOT EXISTS "subscription_id" text;

UPDATE "payment_invoices" pi
SET "subscription_id" = 'SUB0' || substr(up."id", 5)
FROM "user_plans" up
WHERE pi."user_id" = up."user_id" 
  AND pi."created_at" >= up."created_at"
  AND pi."created_at" <= up."expires_at"
  AND pi."subscription_id" IS NULL;

DO $$
DECLARE
    deleted_records integer := 0;
    deleted_orphan_invoices integer := 0;
    deleted_upcoming_invoices integer := 0;
    invoice_ids_to_delete text[];
BEGIN
    SELECT ARRAY_AGG(id) INTO invoice_ids_to_delete
    FROM payment_invoices
    WHERE subscription_id IS NULL OR state = 'UPCOMING';
    
    IF invoice_ids_to_delete IS NOT NULL THEN
        DELETE FROM payment_records
        WHERE invoice_id = ANY(invoice_ids_to_delete);
        GET DIAGNOSTICS deleted_records = ROW_COUNT;
    END IF;
    
    DELETE FROM payment_invoices
    WHERE subscription_id IS NULL;
    GET DIAGNOSTICS deleted_orphan_invoices = ROW_COUNT;
    
    DELETE FROM payment_invoices
    WHERE state = 'UPCOMING';
    GET DIAGNOSTICS deleted_upcoming_invoices = ROW_COUNT;
    
    RAISE NOTICE 'Deleted % payment_records', deleted_records;
    RAISE NOTICE 'Deleted % orphan payment_invoices', deleted_orphan_invoices;
    RAISE NOTICE 'Deleted % UPCOMING payment_invoices', deleted_upcoming_invoices;
END $$;

ALTER TABLE "payment_invoices" ALTER COLUMN "subscription_id" SET NOT NULL;

ALTER TABLE "payment_invoices" ADD COLUMN "due_at" timestamp with time zone;
UPDATE "payment_invoices" SET "due_at" = "billing_at";
ALTER TABLE "payment_invoices" ALTER COLUMN "due_at" SET NOT NULL;

ALTER TABLE "payment_records" ADD COLUMN "outcome" "_payment_outcome" NOT NULL DEFAULT 'SUCCESS';
ALTER TABLE "payment_records" ADD COLUMN "billing_amount" integer NOT NULL DEFAULT 0;
ALTER TABLE "payment_records" ADD COLUMN "credit_amount" integer NOT NULL DEFAULT 0;
ALTER TABLE "payment_records" ADD COLUMN "data" jsonb NOT NULL DEFAULT '{}';

CREATE TEMP TABLE payment_records_aggregated AS
SELECT 
    pr.invoice_id,
    CASE 
        WHEN BOOL_AND(pr.state = 'SUCCEEDED') THEN 'SUCCESS'::_payment_outcome
        ELSE 'FAILURE'::_payment_outcome
    END as outcome,
    COALESCE(SUM(CASE WHEN pr.method_type = 'BILLING_KEY' THEN pr.amount ELSE 0 END), 0) as billing_amount,
    COALESCE(SUM(CASE WHEN pr.method_type = 'CREDIT' THEN pr.amount ELSE 0 END), 0) as credit_amount,
    MIN(pr.created_at) as created_at,
    MIN(pr.id) as first_record_id
FROM payment_records pr
GROUP BY pr.invoice_id;

UPDATE "payment_records" pr
SET "data" = (
    SELECT jsonb_agg(
        jsonb_build_object(
            'id', pr2.id,
            'method_type', pr2.method_type,
            'method_id', pr2.method_id, 
            'state', pr2.state,
            'amount', pr2.amount,
            'receipt_url', pr2.receipt_url,
            'created_at', pr2.created_at
        ) ORDER BY pr2.created_at
    )
    FROM payment_records pr2 
    WHERE pr2.invoice_id = pr.invoice_id
)
WHERE pr.id IN (
    SELECT first_record_id FROM payment_records_aggregated
);

DELETE FROM "payment_records" 
WHERE id NOT IN (
    SELECT first_record_id FROM payment_records_aggregated
);

UPDATE "payment_records" pr
SET 
    "outcome" = pra.outcome,
    "billing_amount" = pra.billing_amount,
    "credit_amount" = pra.credit_amount,
    "created_at" = pra.created_at
FROM payment_records_aggregated pra
WHERE pr.id = pra.first_record_id;

DROP TABLE payment_records_aggregated;

ALTER TABLE "plans" ADD COLUMN "rule" jsonb NOT NULL DEFAULT '{}';
ALTER TABLE "plans" ADD COLUMN "interval" "_plan_interval";

UPDATE "plans" SET "rule" = "rules";

UPDATE "plans" 
SET "interval" = CASE 
    WHEN "id" = 'PL0PLUS' THEN 'MONTHLY'::_plan_interval
    WHEN "id" = 'PL0FULL1MONTH' THEN 'MONTHLY'::_plan_interval
    WHEN "id" = 'PL0FULL1YEAR' THEN 'YEARLY'::_plan_interval
END;

ALTER TABLE "plans" ALTER COLUMN "interval" SET NOT NULL;

ALTER TABLE "subscriptions" ADD CONSTRAINT "subscriptions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "subscriptions" ADD CONSTRAINT "subscriptions_plan_id_plans_id_fk" FOREIGN KEY ("plan_id") REFERENCES "public"."plans"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_billing_keys" ADD CONSTRAINT "user_billing_keys_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_in_app_purchases" ADD CONSTRAINT "user_in_app_purchases_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "payment_invoices" ADD CONSTRAINT "payment_invoices_subscription_id_subscriptions_id_fk" FOREIGN KEY ("subscription_id") REFERENCES "public"."subscriptions"("id") ON DELETE restrict ON UPDATE cascade;

ALTER TABLE "payment_invoices" DROP COLUMN IF EXISTS "billing_at";
ALTER TABLE "payment_records" DROP COLUMN IF EXISTS "method_type";
ALTER TABLE "payment_records" DROP COLUMN IF EXISTS "method_id";
ALTER TABLE "payment_records" DROP COLUMN IF EXISTS "state";
ALTER TABLE "payment_records" DROP COLUMN IF EXISTS "amount";
ALTER TABLE "payment_records" DROP COLUMN IF EXISTS "receipt_url";
ALTER TABLE "plans" DROP COLUMN IF EXISTS "rules";

DO $$
DECLARE
    payment_billing_keys_count integer;
    user_plans_count integer;
    user_billing_keys_count integer;
    subscriptions_count integer;
    remaining_invoices_count integer;
BEGIN
    SELECT COUNT(*) INTO payment_billing_keys_count FROM payment_billing_keys WHERE state = 'ACTIVE';
    SELECT COUNT(*) INTO user_plans_count FROM user_plans;
    SELECT COUNT(*) INTO user_billing_keys_count FROM user_billing_keys;
    SELECT COUNT(*) INTO subscriptions_count FROM subscriptions;
    SELECT COUNT(*) INTO remaining_invoices_count FROM payment_invoices;
    
    IF payment_billing_keys_count != user_billing_keys_count THEN
        RAISE EXCEPTION 'Data migration failed: payment_billing_keys count mismatch (% vs %)', 
            payment_billing_keys_count, user_billing_keys_count;
    END IF;
    
    IF user_plans_count != subscriptions_count THEN
        RAISE EXCEPTION 'Data migration failed: user_plans count mismatch (% vs %)', 
            user_plans_count, subscriptions_count;
    END IF;
    
    RAISE NOTICE 'Migration validation successful:';
    RAISE NOTICE '  - % billing keys migrated', user_billing_keys_count;
    RAISE NOTICE '  - % subscriptions created', subscriptions_count;
    RAISE NOTICE '  - % payment invoices remaining', remaining_invoices_count;
END $$;

DROP TABLE "payment_billing_keys" CASCADE;
DROP TABLE "user_plans" CASCADE;

DROP TYPE "public"."_payment_billing_key_state";
DROP TYPE "public"."_payment_method_type";
DROP TYPE "public"."_payment_record_state";
DROP TYPE "public"."_user_plan_billing_cycle";
DROP TYPE "public"."_user_plan_state";

COMMIT;