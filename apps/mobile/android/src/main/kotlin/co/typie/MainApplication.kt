package co.typie

import android.app.Application
import co.typie.platform.PlatformModule
import com.kakao.sdk.common.KakaoSdk
import com.navercorp.nid.NidOAuth
import io.sentry.kotlin.multiplatform.Sentry

class MainApplication : Application() {
  override fun onCreate() {
    super.onCreate()

    val packageInfo = packageManager.getPackageInfo(packageName, 0)

    Sentry.init { options ->
      options.dsn = Konfig.SENTRY_DSN
      options.sendDefaultPii = true
      options.attachScreenshot = true
      options.release = "$packageName@${packageInfo.versionName}+${packageInfo.longVersionCode}"
    }

    PlatformModule.context = this

    KakaoSdk.init(this, Konfig.KAKAO_NATIVE_APP_KEY)
    NidOAuth.initialize(this, Konfig.NAVER_CLIENT_ID, Konfig.NAVER_CLIENT_SECRET, "타이피")
  }
}
