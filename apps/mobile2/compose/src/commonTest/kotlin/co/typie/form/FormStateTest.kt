package co.typie.form

import androidx.compose.ui.text.input.ImeAction
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class TestLoginForm(scope: TestScope = TestScope()) : FormState(scope) {
  val email = field("") { required(); email() }
  val password = field("") { required(); minLength(6) }
}

class TestProfileForm(nickname: String, bio: String, scope: TestScope = TestScope()) : FormState(scope) {
  val nickname = field(nickname) { required() }
  val bio = field(bio) { maxLength(500) }
}

class FormStateTest {

  @Test
  fun createForm() {
    val form = TestLoginForm()
    assertNotNull(form)
  }

  @Test
  fun fieldInitialValues() {
    val form = TestLoginForm()
    assertEquals("", form.email.value)
    assertEquals("", form.password.value)
  }

  @Test
  fun fieldValueReadWrite() {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    assertEquals("test@test.com", form.email.value)
  }

  @Test
  fun formIsDirtyWhenAnyFieldDirty() {
    val form = TestLoginForm()
    assertFalse(form.isDirty)
    form.email.setValue("changed")
    assertTrue(form.isDirty)
  }

  @Test
  fun formIsNotDirtyWhenAllFieldsClean() {
    val form = TestLoginForm()
    form.email.setValue("changed")
    form.email.setValue("")
    assertFalse(form.isDirty)
  }

  @Test
  fun formIsValidWhenNoErrors() {
    val form = TestLoginForm()
    assertTrue(form.isValid)
  }

  @Test
  fun formIsInvalidWhenFieldHasErrors() {
    val form = TestLoginForm()
    form.email.setErrors(listOf("에러"))
    assertFalse(form.isValid)
  }

  @Test
  fun formErrors() {
    val form = TestLoginForm()
    form.email.setErrors(listOf("이메일 에러"))
    form.password.setErrors(listOf("비밀번호 에러"))
    val errors = form.errors
    assertEquals(listOf("이메일 에러"), errors[form.email])
    assertEquals(listOf("비밀번호 에러"), errors[form.password])
  }

  @Test
  fun formReset() {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    form.password.setValue("secret")
    form.email.setErrors(listOf("에러"))

    form.reset()

    assertEquals("", form.email.value)
    assertEquals("", form.password.value)
    assertEquals(emptyList(), form.email.errors)
    assertFalse(form.isDirty)
  }

  @Test
  fun dynamicInitialValues() {
    val form = TestProfileForm(nickname = "기존닉네임", bio = "기존소개")
    assertEquals("기존닉네임", form.nickname.value)
    assertEquals("기존소개", form.bio.value)
  }

  @Test
  fun validateAllFieldsOnSubmit() = runTest {
    val form = TestLoginForm(this)
    val isValid = form.validateAll()
    assertFalse(isValid)
    assertEquals(listOf("필수 항목입니다"), form.email.errors)
    assertEquals(listOf("필수 항목입니다"), form.password.errors)
  }

  @Test
  fun validateAllPassesWhenValid() = runTest {
    val form = TestLoginForm(this)
    form.email.setValue("test@test.com")
    form.password.setValue("secret123")
    val isValid = form.validateAll()
    assertTrue(isValid)
    assertEquals(emptyList(), form.email.errors)
    assertEquals(emptyList(), form.password.errors)
  }

  @Test
  fun defaultValidateOnPropagates() {
    val scope = TestScope()
    class OnBlurForm : FormState(scope, defaultValidateOn = ValidateOn.Blur) {
      val name = field("") { required() }
      val overridden = field("") { required(); validateOn(ValidateOn.Change) }
    }

    val form = OnBlurForm()
    assertTrue(ValidateOn.Blur in form.name.rulesByTiming)
    assertTrue(ValidateOn.Change in form.overridden.rulesByTiming)
  }

  @Test
  fun perRuleValidateOn() = runTest {
    val form = object : FormState(this) {
      val name = field("") {
        required("닉네임을 입력해주세요.")
        validateOn(ValidateOn.Change) {
          maxLength(20, "닉네임은 20자를 넘을 수 없어요.")
        }
      }
    }

    // OnChange rule triggers on value change
    form.name.setValue("a".repeat(21))
    testScheduler.advanceUntilIdle()
    assertEquals(listOf("닉네임은 20자를 넘을 수 없어요."), form.name.errors)

    // OnSubmit rule (required) doesn't trigger on change
    form.name.setValue("")
    testScheduler.advanceUntilIdle()
    assertEquals(emptyList(), form.name.errors)

    // On submit, all rules trigger
    val valid = form.validateAll()
    assertFalse(valid)
    assertEquals(listOf("닉네임을 입력해주세요."), form.name.errors)
  }

  @Test
  fun validateReturnsFalseAndFocusesFirstError() = runTest {
    val form = TestLoginForm(this)
    val valid = form.validate()
    assertFalse(valid)
    assertEquals(listOf("필수 항목입니다"), form.email.errors)
  }

  @Test
  fun validateReturnsTrueWhenValid() = runTest {
    val form = TestLoginForm(this)
    form.email.setValue("test@test.com")
    form.password.setValue("secret123")
    val valid = form.validate()
    assertTrue(valid)
  }

  // --- Validation Timing Tests ---

  @Test
  fun onBlurTriggersValidation() = runTest {
    class BlurForm : FormState(this) {
      val email = field("") {
        required()
        validateOn(ValidateOn.Blur)
      }
    }

    val form = BlurForm()

    form.email.onBlur()
    testScheduler.advanceUntilIdle()

    assertEquals(listOf("필수 항목입니다"), form.email.errors)
  }

  @Test
  fun onBlurDoesNotTriggerForOnSubmitFields() = runTest {
    val form = TestLoginForm(this)
    form.email.onBlur()
    testScheduler.advanceUntilIdle()

    assertEquals(emptyList(), form.email.errors)
  }

  @Test
  fun onChangeTriggersValidation() = runTest {
    class ChangeForm : FormState(this) {
      val email = field("") {
        email()
        validateOn(ValidateOn.Change)
      }
    }

    val form = ChangeForm()

    form.email.setValue("invalid")
    testScheduler.advanceUntilIdle()

    assertEquals(listOf("올바른 이메일 형식을 입력해주세요"), form.email.errors)
  }

  @Test
  fun onSubmitAutoClearsErrors() {
    val form = TestLoginForm()
    form.email.setErrors(listOf("이전 에러"))
    form.email.setValue("new value")
    assertEquals(emptyList(), form.email.errors)
  }

  @Test
  fun onChangeDoesNotAutoClear() = runTest {
    class ChangeForm : FormState(this) {
      val email = field("") {
        required()
        validateOn(ValidateOn.Change)
      }
    }

    val form = ChangeForm()

    form.email.setErrors(listOf("이전 에러"))
    form.email.setValue("new value")
    assertEquals(listOf("이전 에러"), form.email.errors)

    testScheduler.advanceUntilIdle()
    assertEquals(emptyList(), form.email.errors)
  }

  @Test
  fun onChangeDeferRulesDebounced() = runTest {
    var deferCallCount = 0
    val form = object : FormState(this) {
      val email = field("") {
        defer {
          deferCallCount++
          null
        }
        validateOn(ValidateOn.Change)
      }
    }

    form.email.setValue("a")
    form.email.setValue("ab")
    form.email.setValue("abc")
    testScheduler.advanceUntilIdle()

    assertEquals(1, deferCallCount)
  }

  @Test
  fun onChangeSyncAndDeferCombined() = runTest {
    var deferCallCount = 0
    val form = object : FormState(this) {
      val email = field("") {
        email()
        defer {
          deferCallCount++
          if (it == "taken@test.com") "이미 사용 중" else null
        }
        validateOn(ValidateOn.Change)
      }
    }

    form.email.setValue("invalid")
    testScheduler.advanceUntilIdle()
    assertEquals(listOf("올바른 이메일 형식을 입력해주세요"), form.email.errors)

    form.email.setValue("taken@test.com")
    testScheduler.advanceUntilIdle()
    assertEquals(listOf("이미 사용 중"), form.email.errors)
  }

  @Test
  fun isLastField_returns_true_for_last_registered_field() {
    val form = object : FormState(TestScope()) {
      val first = field("")
      val second = field("")
    }
    assertFalse(form.isLastField(form.first))
    assertTrue(form.isLastField(form.second))
  }

  @Test
  fun imeActionFor_returns_next_for_non_last_field() {
    val form = object : FormState(TestScope()) {
      val first = field("")
      val second = field("")
    }
    assertEquals(ImeAction.Next, form.imeActionFor(form.first))
    assertEquals(ImeAction.Done, form.imeActionFor(form.second))
  }

  @Test
  fun field_has_form_back_reference() {
    val form = object : FormState(TestScope()) {
      val first = field("")
    }
    assertEquals(form, form.first.form)
  }

  // --- initialValue setter ---

  @Test
  fun initialValueSetterUpdatesValueAndResetsState() {
    val form = TestProfileForm(nickname = "기존닉네임", bio = "기존소개")
    form.nickname.setValue("변경됨")
    assertTrue(form.nickname.isDirty)

    form.nickname.initialValue = "서버값"
    assertEquals("서버값", form.nickname.value)
    assertEquals("서버값", form.nickname.initialValue)
    assertFalse(form.nickname.isDirty)
    assertFalse(form.nickname.isTouched)
    assertEquals(emptyList(), form.nickname.errors)
  }

  @Test
  fun initialValueSetterDoesNotAffectOtherFields() {
    val form = TestProfileForm(nickname = "닉", bio = "소개")
    form.bio.setValue("변경됨")
    assertTrue(form.bio.isDirty)

    form.nickname.initialValue = "새닉네임"
    assertTrue(form.bio.isDirty)
  }

  // --- commit ---

  @Test
  fun commitUpdatesInitialValueToCurrentValue() {
    val form = TestProfileForm(nickname = "기존", bio = "기존소개")
    form.nickname.setValue("변경됨")
    assertTrue(form.nickname.isDirty)

    form.nickname.commit()
    assertEquals("변경됨", form.nickname.initialValue)
    assertFalse(form.nickname.isDirty)
  }

  @Test
  fun formCommitCommitsAllFields() {
    val form = TestProfileForm(nickname = "기존", bio = "기존소개")
    form.nickname.setValue("새닉네임")
    form.bio.setValue("새소개")
    assertTrue(form.isDirty)

    form.commit()
    assertFalse(form.isDirty)
    assertEquals("새닉네임", form.nickname.initialValue)
    assertEquals("새소개", form.bio.initialValue)
  }

  @Test
  fun resetAfterInitialValueChangeUsesNewInitialValue() {
    val form = TestProfileForm(nickname = "기존", bio = "기존소개")
    form.nickname.initialValue = "서버값"
    form.nickname.setValue("사용자입력")
    assertTrue(form.nickname.isDirty)

    form.nickname.reset()
    assertEquals("서버값", form.nickname.value)
    assertFalse(form.nickname.isDirty)
  }
}
