package co.typie.editor

import co.typie.di.PlatformContext
import co.typie.editor.ffi.EditorHost
import co.typie.editor.ffi.IosEditorHost
import kotlinx.cinterop.ObjCObjectVar
import kotlinx.cinterop.alloc
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.value
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.suspendCancellableCoroutine
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single
import platform.Foundation.NSBundle
import platform.Foundation.NSData
import platform.Foundation.NSError
import platform.Foundation.create
import swiftPMImport.co.typie.compose.NativeEditorHost

@Module
actual class EditorModule {
  @Single
  actual fun editorHost(ctx: PlatformContext): EditorHost {
    val native = NativeEditorHost()

    runBlocking {
      suspendCancellableCoroutine { cont ->
        native.createWithCompletion { error ->
          if (error != null) cont.resumeWith(Result.failure(EditorException(error.localizedDescription)))
          else cont.resumeWith(Result.success(Unit))
        }
      }
    }

    val path = NSBundle.mainBundle.pathForResource("icu", "zst")!!
    val icu = NSData.create(contentsOfFile = path)!!
    memScoped {
      val err = alloc<ObjCObjectVar<NSError?>>()
      native.loadIcuDataWithData(icu, err.ptr)
      err.value?.let { throw EditorException(it.localizedDescription) }
    }

    return IosEditorHost(native)
  }
}
