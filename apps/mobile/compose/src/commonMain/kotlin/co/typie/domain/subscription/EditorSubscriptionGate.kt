package co.typie.domain.subscription

/**
 * 에디터가 읽기 전용이어야 하는지. 서버 잠금(document.locked) 또는 권한 없음이면 true. Unknown은 낙관적으로 편집 허용 — 서버 거부는 백스톱이
 * 처리한다.
 */
fun editorIsReadOnly(documentLocked: Boolean, entitlement: Entitlement): Boolean =
  documentLocked || !entitlement.grantsAccess()

/** 권한이 있으면(Unknown 포함) push를 시도해도 된다. */
fun shouldAttemptPush(entitlement: Entitlement): Boolean = entitlement.grantsAccess()
