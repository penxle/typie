ALTER TABLE "payment_invoices" ADD COLUMN "last_attempted_at" timestamp with time zone;--> statement-breakpoint
UPDATE "payment_invoices" pi
SET "last_attempted_at" = pr.max_created_at
FROM (SELECT invoice_id, MAX(created_at) AS max_created_at FROM payment_records GROUP BY invoice_id) pr
WHERE pr.invoice_id = pi.id;