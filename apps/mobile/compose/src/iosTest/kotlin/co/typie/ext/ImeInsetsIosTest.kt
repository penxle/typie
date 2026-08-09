@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.ext

import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.runtime.AbstractApplier
import androidx.compose.runtime.Composition
import androidx.compose.runtime.Recomposer
import androidx.compose.ui.unit.Density
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.cinterop.useContents
import kotlinx.coroutines.test.runTest
import platform.CoreGraphics.CGRectMake
import platform.Foundation.NSNotificationCenter
import platform.Foundation.NSValue
import platform.UIKit.UIKeyboardFrameEndUserInfoKey
import platform.UIKit.UIKeyboardWillChangeFrameNotification
import platform.UIKit.UIKeyboardWillHideNotification
import platform.UIKit.UIScreen
import platform.UIKit.valueWithCGRect
import swiftPMImport.co.typie.compose.SoftwareKeyboardPresentationBridge

class ImeInsetsIosTest {
  @Test
  fun keyboardWillHideClearsPresentationFrameOverride() = runTest {
    var insets: WindowInsets? = null
    val composition = Composition(UnitApplier(), Recomposer(coroutineContext))
    try {
      composition.setContent { insets = rememberTrustedImeInsets() }

      val (screenWidth, screenHeight) =
        UIScreen.mainScreen.bounds.useContents { size.width to size.height }
      val keyboardHeight = 200.0
      val keyboardFrame =
        CGRectMake(
          x = 0.0,
          y = screenHeight - keyboardHeight,
          width = screenWidth,
          height = keyboardHeight,
        )
      val center = NSNotificationCenter.defaultCenter
      val bridge = SoftwareKeyboardPresentationBridge()
      center.postNotificationName(
        aName = UIKeyboardWillChangeFrameNotification,
        `object` = bridge,
        userInfo = mapOf(UIKeyboardFrameEndUserInfoKey to NSValue.valueWithCGRect(keyboardFrame)),
      )

      assertEquals(200, requireNotNull(insets).getBottom(Density(1f)))

      center.postNotificationName(
        aName = UIKeyboardWillHideNotification,
        `object` = null,
        userInfo = null,
      )

      assertEquals(0, requireNotNull(insets).getBottom(Density(1f)))
    } finally {
      composition.dispose()
    }
  }
}

private class UnitApplier : AbstractApplier<Unit>(Unit) {
  override fun insertBottomUp(index: Int, instance: Unit) = Unit

  override fun insertTopDown(index: Int, instance: Unit) = Unit

  override fun move(from: Int, to: Int, count: Int) = Unit

  override fun onClear() = Unit

  override fun remove(index: Int, count: Int) = Unit
}
