package co.typie.entity_transfer

fun PasteError.toMessage(): String = when (this) {
  PasteError.SiteMismatch -> "이 위치에는 붙여넣을 수 없어요."
  PasteError.CircularReference -> "자기 자신 또는 하위 항목 안에는 붙여넣을 수 없어요."
  PasteError.SourceNotFound -> "붙여넣을 항목을 찾을 수 없어요."
  PasteError.CharacterCountLimitExceeded -> "현재 플랜의 글자 수 제한을 초과했어요."
  PasteError.BlobSizeLimitExceeded -> "현재 플랜의 파일 크기 제한을 초과했어요."
}
