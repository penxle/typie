package co.typie.domain.subscription

/**
 * 에디터가 읽기 전용이어야 하는지. 서버 잠금(document.locked) 또는 구독 만료(Expired)면 true. Unknown은 낙관적으로 편집 허용(스펙 §3.1)
 * — 서버 거부는 백스톱이 처리한다.
 */
fun editorIsReadOnly(documentLocked: Boolean, entitlement: Entitlement): Boolean =
  documentLocked || entitlement is Entitlement.Expired

/** Expired가 아니면 push를 시도해도 된다. Unknown은 낙관적으로 통과. */
fun shouldAttemptPush(entitlement: Entitlement): Boolean = entitlement !is Entitlement.Expired
