#!/usr/bin/env node

// 실행 봉인.
//
// 원래 목적: 구 renewal:cancel 크론이 로컬 상태만 보고 잘못 EXPIRED 처리한 IAP 구독을 스토어와
// 대조해 되살리는 1회성 백필이었다. 판정·복구 모두 구 만료 컬럼을 원본으로 삼았는데, 권한이
// 상태 + 주기 컬럼에서 나오게 되면서 그 전제가 사라졌다.
//
// 같은 복구가 다시 필요하면 이 파일을 되살리지 말고 권한 판정·재조정 경로 기준으로 새로 쓴다.
// 원문은 git 이력에 있다.

throw new Error('superseded by entitlement split');
