package co.typie

import android.app.Application
import co.typie.platform.PlatformModule
import com.kakao.sdk.common.KakaoSdk
import com.navercorp.nid.NidOAuth

class MainApplication : Application() {
  override fun onCreate() {
    super.onCreate()

    PlatformModule.context = this

    KakaoSdk.init(this, Konfig.KAKAO_NATIVE_APP_KEY)
    NidOAuth.initialize(this, Konfig.NAVER_CLIENT_ID, Konfig.NAVER_CLIENT_SECRET, "타이피")
  }
}
