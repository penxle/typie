CREATE TYPE "public"."_prism_credit_pack" AS ENUM('P100', 'P330', 'P690', 'P1440', 'P3000');
CREATE TYPE "public"."_prism_credit_purchase_channel" AS ENUM('BILLING_KEY');
CREATE TYPE "public"."_prism_credit_purchase_state" AS ENUM('PENDING', 'PAID', 'FAILED');
CREATE TYPE "public"."_prism_credit_refund_kind" AS ENUM('WITHDRAWAL', 'REMAINDER');
CREATE TYPE "public"."_prism_credit_refund_method" AS ENUM('PG_CANCEL', 'MANUAL');
CREATE TYPE "public"."_prism_credit_refund_state" AS ENUM('PENDING', 'DONE');
ALTER TYPE "public"."_prism_credit_entry_kind" ADD VALUE 'PURCHASE';
ALTER TYPE "public"."_prism_credit_entry_kind" ADD VALUE 'BONUS';
ALTER TYPE "public"."_prism_credit_entry_kind" ADD VALUE 'REFUND_OUT';
CREATE TABLE "prism_credit_purchases" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"pack" "_prism_credit_pack" NOT NULL,
	"price" integer NOT NULL,
	"credits" integer NOT NULL,
	"bonus_credits" integer NOT NULL,
	"channel" "_prism_credit_purchase_channel" NOT NULL,
	"billing_key_type" "_billing_key_type" NOT NULL,
	"payment_key" text NOT NULL,
	"state" "_prism_credit_purchase_state" NOT NULL,
	"paid_at" timestamp with time zone,
	"data" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_credit_purchases_payment_key_unique" UNIQUE("payment_key")
);

CREATE TABLE "prism_credit_refunds" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"kind" "_prism_credit_refund_kind" NOT NULL,
	"purchase_id" text,
	"amount" integer NOT NULL,
	"method" "_prism_credit_refund_method" NOT NULL,
	"state" "_prism_credit_refund_state" NOT NULL,
	"actor_id" text NOT NULL,
	"note" text NOT NULL,
	"data" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "prism_credit_purchases" ADD CONSTRAINT "prism_credit_purchases_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_credit_refunds" ADD CONSTRAINT "prism_credit_refunds_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_credit_refunds" ADD CONSTRAINT "prism_credit_refunds_purchase_id_prism_credit_purchases_id_fk" FOREIGN KEY ("purchase_id") REFERENCES "public"."prism_credit_purchases"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_credit_refunds" ADD CONSTRAINT "prism_credit_refunds_actor_id_users_id_fk" FOREIGN KEY ("actor_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "prism_credit_purchases_user_id_created_at_index" ON "prism_credit_purchases" USING btree ("user_id","created_at");
CREATE INDEX "prism_credit_purchases_pending_index" ON "prism_credit_purchases" USING btree ("created_at") WHERE "prism_credit_purchases"."state" = 'PENDING';
CREATE INDEX "prism_credit_refunds_user_id_created_at_index" ON "prism_credit_refunds" USING btree ("user_id","created_at");
CREATE INDEX "prism_credit_refunds_purchase_id_index" ON "prism_credit_refunds" USING btree ("purchase_id");