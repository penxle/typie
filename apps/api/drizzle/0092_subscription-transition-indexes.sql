-- 적용 전 preflight (배포 runbook 필수 게이트): 유저당 WILL_ACTIVATE 가 2건 이상이면 아래 유니크 인덱스 생성이
-- 실패하며 마이그레이션 전체가 원자 롤백된다(의도된 배포 차단). 사전 검사:
--   SELECT user_id, count(*) FROM subscriptions WHERE state = 'WILL_ACTIVATE' GROUP BY user_id HAVING count(*) > 1;
-- 중복 발견 시 자동 정리하지 않는다 — 어느 예약을 남길지는 사업 판단이므로 건별로 수동 정리 후 재적용한다.
CREATE UNIQUE INDEX "subscriptions_will_activate_user_id_index" ON "subscriptions" USING btree ("user_id") WHERE "subscriptions"."state" = 'WILL_ACTIVATE';
CREATE INDEX "subscriptions_will_activate_starts_at_index" ON "subscriptions" USING btree ("starts_at") WHERE "subscriptions"."state" = 'WILL_ACTIVATE';
CREATE INDEX "subscriptions_will_expire_expires_at_index" ON "subscriptions" USING btree ("expires_at") WHERE "subscriptions"."state" = 'WILL_EXPIRE';
