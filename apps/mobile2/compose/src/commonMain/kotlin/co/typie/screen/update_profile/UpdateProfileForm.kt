package co.typie.screen.update_profile

import co.typie.form.FormState
import co.typie.form.maxLength

class UpdateProfileForm(
  initialName: String,
  initialAvatarId: String,
) : FormState() {
  val name = field(initialName) {
    required("닉네임을 입력해주세요.")
    maxLength(20, "닉네임은 20자를 넘을 수 없어요.")
  }

  val avatarId = field(initialAvatarId) {
    required("프로필 사진을 선택해주세요.")
  }
}
