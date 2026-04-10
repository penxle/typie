package co.typie.screen.settings.delete_user

import co.typie.result.DEFAULT_ERROR_MESSAGE

internal fun deleteUserNoticeItems(): List<String> {
  return listOf(
    "- 작성한 모든 글과 데이터는 탈퇴와 함께 삭제되며 재가입시에도 복구할 수 없어요.",
    "- 이용중인 스페이스 주소는 다시 이용할 수 없어요. 스페이스 주소를 다시 사용할 계획이라면, 탈퇴 전 기존 주소를 변경해주세요.",
    "- 남은 이용권 기간은 탈퇴와 함께 소멸되며, 환불은 별도로 제공되지 않아요.",
    "- 스토어에서 이용권을 구매했을 경우, 구독 취소 처리는 스토어 규정상 스토어 내 설정에서 직접 진행해야 해요.",
  )
}

internal fun deleteUserValidationMessage(isAcknowledged: Boolean): String? {
  return if (isAcknowledged) null else "유의사항을 모두 확인해주세요."
}

internal fun deleteUserErrorMessage(code: String, message: String?): String {
  return when (code) {
    "overdue_invoices_exist" -> "미납된 결제가 있어 회원 탈퇴를 진행할 수 없어요. 결제 상태를 먼저 확인해주세요."
    else -> message ?: DEFAULT_ERROR_MESSAGE
  }
}
