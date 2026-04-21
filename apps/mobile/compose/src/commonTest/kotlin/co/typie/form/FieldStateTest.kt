package co.typie.form

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class FieldStateTest {

  private fun stringField(
    initialValue: String = "",
    rules: List<Rule<String>> = emptyList(),
    deferredRules: List<DeferredRule<String>> = emptyList(),
    validateOn: ValidateOn = ValidateOn.Submit,
  ) =
    FieldState(
      initialValue = initialValue,
      rulesByTiming = if (rules.isEmpty()) emptyMap() else mapOf(validateOn to rules),
      deferredRules = deferredRules,
    )

  @Test
  fun initialState() {
    val field = stringField()
    assertEquals("", field.value)
    assertEquals(emptyList(), field.errors)
    assertFalse(field.isDirty)
    assertFalse(field.isTouched)
    assertFalse(field.isValidating)
  }

  @Test
  fun setValue() {
    val field = stringField()
    field.setValue("hello")
    assertEquals("hello", field.value)
  }

  @Test
  fun isDirtyWhenValueChanges() {
    val field = stringField(initialValue = "")
    assertFalse(field.isDirty)
    field.setValue("changed")
    assertTrue(field.isDirty)
  }

  @Test
  fun isDirtyResetsWhenBackToInitial() {
    val field = stringField(initialValue = "original")
    field.setValue("changed")
    assertTrue(field.isDirty)
    field.setValue("original")
    assertFalse(field.isDirty)
  }

  @Test
  fun setValueClearsErrors() {
    val field = stringField()
    field.setErrors(listOf("에러"))
    assertEquals(listOf("에러"), field.errors)
    field.setValue("new")
    assertEquals(emptyList(), field.errors)
  }

  @Test
  fun setErrorsManually() {
    val field = stringField()
    field.setErrors(listOf("서버 에러"))
    assertEquals(listOf("서버 에러"), field.errors)
  }

  @Test
  fun onBlurSetsTouched() {
    val field = stringField()
    assertFalse(field.isTouched)
    field.onBlur()
    assertTrue(field.isTouched)
  }

  @Test
  fun reset() {
    val field = stringField(initialValue = "init")
    field.setValue("changed")
    field.setErrors(listOf("에러"))
    field.onBlur()
    assertTrue(field.isDirty)
    assertTrue(field.isTouched)

    field.reset()
    assertEquals("init", field.value)
    assertEquals(emptyList(), field.errors)
    assertFalse(field.isDirty)
    assertFalse(field.isTouched)
  }

  @Test
  fun rollback_restores_last_committed_value() {
    val field = stringField(initialValue = "init")
    field.setValue("saved")
    field.commit()

    field.setValue("draft")
    field.setErrors(listOf("에러"))
    field.onBlur()

    field.rollback()

    assertEquals("saved", field.value)
    assertEquals(emptyList(), field.errors)
    assertFalse(field.isDirty)
    assertFalse(field.isTouched)
  }

  @Test
  fun commit_updates_rollback_target() {
    val field = stringField(initialValue = "init")
    field.setValue("first")
    field.commit()
    field.setValue("second")
    field.commit()
    field.setValue("draft")

    field.rollback()

    assertEquals("second", field.value)
    assertFalse(field.isDirty)
  }

  @Test
  fun syncFromSource_updates_pristine_field_value_and_initial() {
    val field = stringField(initialValue = "init")

    field.syncFromSource("server")

    assertEquals("server", field.initialValue)
    assertEquals("server", field.value)
    assertFalse(field.isDirty)
  }

  @Test
  fun syncFromSource_preserves_dirty_value_while_updating_initial() {
    val field = stringField(initialValue = "init")
    field.setValue("draft")

    field.syncFromSource("server")

    assertEquals("server", field.initialValue)
    assertEquals("draft", field.value)
    assertTrue(field.isDirty)
  }

  @Test
  fun syncFromSource_clears_dirty_when_source_catches_up() {
    val field = stringField(initialValue = "init")
    field.setValue("draft")

    field.syncFromSource("draft")

    assertEquals("draft", field.initialValue)
    assertEquals("draft", field.value)
    assertFalse(field.isDirty)
  }

  @Test
  fun destructuring() {
    val field = stringField(initialValue = "hello")
    field.setErrors(listOf("에러"))
    val (value, onChange, errors) = field
    assertEquals("hello", value)
    assertEquals(listOf("에러"), errors)

    onChange("world")
    assertEquals("world", field.value)
  }
}
