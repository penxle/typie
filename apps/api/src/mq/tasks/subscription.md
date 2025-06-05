# 구독 관련 전체 정리

## 관련 테이블

- `Subscriptions`
- `PaymentInvoices`
- `PaymentRecords`
- `Plans`
- `UserBillingKeys`
- `UserInAppPurchases`
- `UserPaymentCredits`

## 관련 상태

### SubscriptionState

- `ACTIVE`: 구독이 정상적으로 활성화되어 있는 상태
- `WILL_ACTIVATE`: `startsAt` 이후 구독이 활성화될 예정인 상태
- `WILL_EXPIRE`: `expiresAt` 이후 구독이 종료될 예정인 상태
- `IN_GRACE_PERIOD`: `expiresAt` 이 지났지만 아직 구독이 종료되진 않은 상태
- `EXPIRED`: 구독이 종료된 상태

### PaymentInvoiceState

- `UPCOMING`: 결제 예정인 상태
- `PAID`: 결제 완료된 상태
- `OVERDUE`: 결제 기한이 지났지만 아직 결제가 완료되지 않은 상태
- `CANCELED`: 결제가 취소된 상태

### PaymentRecordState

- `SUCCEEDED`: 결제 완료된 상태
- `FAILED`: 결제 실패한 상태

## 빌링 키 플로우

### 첫 구독 (요청)

1. 구독할 `Plans` 정보를 받음
2. `Subscriptions` 레코드 생성
   a. `startsAt` = 현재 시간
   b. `expiresAt` = `startsAt` + `Plans.interval`
   c. `SubscriptionState` = `ACTIVE`
3. `PaymentInvoices` 레코드 생성
   a. `amount` = `Plans.fee`
   b. `state` = `UPCOMING`
4. 생성된 `PaymentInvoices` 로 결제 요청
5. 결제되지 않았을 경우
   a. 에러 반환 후 전체 트랜잭션 롤백
6. 결제되었을 경우
   a. `PaymentInvoices.state` 를 `PAID` 로 변경

### 플랜 변경 (요청)

1. 새로 구독할 `Plans` 정보를 받음
2. 기존 `Subscriptions.state` 가 `ACTIVE` 일 경우 `WILL_EXPIRE` 로 업데이트
3. 새 `Subscriptions` 레코드 생성
   a. `startsAt` = 기존 `Subscriptions.expiresAt`
   b. `expiresAt` = `startsAt` + `Plans.interval`
   c. `SubscriptionState` = `WILL_ACTIVATE`

### 플랜 변경의 취소 (요청)

1. 플랜 변경의 취소 요청을 받음
2. 기존 `Subscriptions.state` 가 `WILL_EXPIRE` 일 경우 `ACTIVE` 로 업데이트
3. `Subscriptions.state` 가 `WILL_ACTIVATE` 인 레코드 삭제

### 구독 취소 (요청)

1. 구독 취소 요청을 받음
2. 기존 `Subscriptions.state` 가 `ACTIVE` 일 경우 `WILL_EXPIRE` 로 업데이트
3. `Subscriptions.state` 가 `WILL_ACTIVATE` 인 레코드 삭제

### 구독 취소의 취소 (요청)

1. 구독 취소의 취소 요청을 받음
2. 기존 `Subscriptions.state` 가 `WILL_EXPIRE` 일 경우 `ACTIVE` 로 업데이트

### 자동 갱신 (크론, 첫 시도)

1. 매일 오전 10시에 실행
2. `Subscriptions.expiresAt` <= `now()` && `Subscriptions.state` = `ACTIVE` 인 경우 첫 시도 대상
3. `PaymentInvoices` 레코드 생성
   a. `amount` = `Plans.fee`
   b. `dueAt` = `Subscriptions.expiresAt`
   c. `state` = `UPCOMING`
4. 생성된 `PaymentInvoices` 로 결제
5. 결제되지 않았을 경우
   a. `Subscriptions.state` 를 `IN_GRACE_PERIOD` 로 업데이트
   b. `PaymentInvoices.state` 를 `OVERDUE` 로 업데이트
6. 결제되었을 경우
   a. `Subscriptions.expiresAt` 을 `Subscriptions.expiresAt` + `Plans.interval` 로 업데이트
   b. `PaymentInvoices.state` 를 `PAID` 로 변경

### 자동 갱신 (크론, 재시도)

1. 매일 오전 10시에 실행
2. `PaymentInvoices.state` = `OVERDUE` 인 경우 재시도 대상
3. 해당 `PaymentInvoices` 로 결제 시도
4. 결제되지 않았고, `Subscriptions.expiresAt` < `now()` - `SUBSCRIPTION_GRACE_DAYS` 일 경우
   a. `PaymentInvoices.state` 를 `CANCELED` 로 업데이트
   b. `Subscriptions.state` 를 `EXPIRED` 로 업데이트
5. 결제되었을 경우
   a. `PaymentInvoices` 업데이트 (`state` = `PAID`)
   b. `Subscriptions.state` 를 `ACTIVE` 로 업데이트
   c. `Subscriptions.expiresAt` 을 `Subscriptions.expiresAt` + `Plans.interval` 로 업데이트

### 플랜 변경 처리 (크론, 첫 시도)

1. 매일 오전 10시에 실행
2. `Subscriptions.state` = `WILL_ACTIVATE` 이고 `Subscriptions.startsAt` <= `now()` 인 경우 대상
3. `PaymentInvoices` 레코드 생성
   a. `amount` = `Plans.fee`
   b. `state` = `UPCOMING`
4. 생성된 `PaymentInvoices` 로 결제
5. 결제되지 않았을 경우
   a. `Subscriptions.state` 를 `IN_GRACE_PERIOD` 로 업데이트
   b. `Subscriptions.expiresAt` 을 `Subscriptions.startsAt` 으로 업데이트
   c. `PaymentInvoices.state` 를 `OVERDUE` 로 업데이트
6. 결제되었을 경우
   a. `Subscriptions.state` 를 `ACTIVE` 로 업데이트
   b. `PaymentInvoices.state` 를 `PAID` 로 업데이트

### 플랜 취소 처리 (크론)

1. 매일 오전 10시에 실행
2. `Subscriptions.state` = `WILL_EXPIRE` 이고 `Subscriptions.expiresAt` <= `now()` 인 경우 대상
3. `Subscriptions.state` 를 `EXPIRED` 로 업데이트
