@file:OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)

package co.typie.editor

import co.typie.di.PlatformContext
import co.typie.editor.ffi.BackendKind
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.IosEditorHost
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import kotlinx.coroutines.runBlocking
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single
import platform.Foundation.NSBundle
import platform.Foundation.NSData
import platform.Foundation.create
import platform.posix.memcpy

@Module
actual class EditorModule {
  @Single
  actual fun editorHost(ctx: PlatformContext): EditorHost {
    val path = NSBundle.mainBundle.pathForResource("icu", "zst")!!
    val nsData = NSData.create(contentsOfFile = path)!!
    val icuData = ByteArray(nsData.length.toInt()).apply {
      usePinned { memcpy(it.addressOf(0), nsData.bytes, nsData.length) }
    }

    return runBlocking { IosEditorHost.create(BackendKind.Gpu, icuData) }
  }
}
