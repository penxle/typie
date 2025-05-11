import 'package:firebase_core/firebase_core.dart';
import 'package:kakao_flutter_sdk_user/kakao_flutter_sdk_user.dart';
import 'package:naver_login_sdk/naver_login_sdk.dart';
import 'package:typie/env.dart';
import 'package:typie/firebase/options.dart';

Future<void> configureStaticServices() async {
  await Firebase.initializeApp(options: DefaultFirebaseOptions.currentPlatform);

  KakaoSdk.init(nativeAppKey: Env.kakaoNativeAppKey);

  await NaverLoginSDK.initialize(clientId: Env.naverClientId, clientSecret: Env.naverClientSecret, urlScheme: 'typie');
}
