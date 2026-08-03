package co.typie.domain.auth

/** 저장된 siteId가 현 유저의 사이트 목록에 없으면 첫 사이트로 폴백한다 (웹 대시보드 레이아웃과 동일 규칙). */
fun resolveActiveSiteId(storedSiteId: String?, availableSiteIds: List<String>): String? =
  if (storedSiteId != null && storedSiteId in availableSiteIds) storedSiteId
  else availableSiteIds.firstOrNull()
