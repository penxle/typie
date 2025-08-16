ALTER TABLE "referral_codes" ADD CONSTRAINT "referral_codes_user_id_unique" UNIQUE("user_id");
ALTER TABLE "referrals" ADD CONSTRAINT "referrals_referee_id_unique" UNIQUE("referee_id");