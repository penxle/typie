ALTER TABLE "post_paywall_purchases" DROP CONSTRAINT "post_paywall_purchases_paywall_id_post_paywalls_id_fk";

ALTER TABLE "post_paywall_purchases" ALTER COLUMN "paywall_id" DROP NOT NULL;
ALTER TABLE "post_paywall_purchases" ADD CONSTRAINT "post_paywall_purchases_paywall_id_post_paywalls_id_fk" FOREIGN KEY ("paywall_id") REFERENCES "public"."post_paywalls"("id") ON DELETE set null ON UPDATE cascade;