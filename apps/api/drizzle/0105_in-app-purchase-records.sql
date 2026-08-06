CREATE TYPE "public"."_in_app_purchase_record_state" AS ENUM('PAID', 'REFUNDED');
CREATE TABLE "in_app_purchase_records" (
	"id" text PRIMARY KEY NOT NULL,
	"store" "_in_app_purchase_store" NOT NULL,
	"identifier" text NOT NULL,
	"user_id" text NOT NULL,
	"product_id" text,
	"state" "_in_app_purchase_record_state" NOT NULL,
	"amount" numeric NOT NULL,
	"currency" text NOT NULL,
	"refunded_amount" numeric,
	"purchased_at" timestamp with time zone NOT NULL,
	"refunded_at" timestamp with time zone,
	"data" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "in_app_purchase_records_store_identifier_unique" UNIQUE("store","identifier")
);

ALTER TABLE "in_app_purchase_records" ADD CONSTRAINT "in_app_purchase_records_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "in_app_purchase_records_user_id_index" ON "in_app_purchase_records" USING btree ("user_id");