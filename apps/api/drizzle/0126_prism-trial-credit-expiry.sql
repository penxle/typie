ALTER TYPE "public"."_prism_credit_entry_kind" ADD VALUE 'EXPIRE';
ALTER TABLE "prism_credit_entries" ADD COLUMN "expires_at" timestamp with time zone;
CREATE INDEX "prism_credit_entries_expires_at_index" ON "prism_credit_entries" USING btree ("expires_at") WHERE "prism_credit_entries"."expires_at" is not null;