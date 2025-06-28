import 'package:firebase_core/firebase_core.dart';
import 'package:flutter/services.dart';
import 'package:jiffy/jiffy.dart';
import 'package:kakao_flutter_sdk_user/kakao_flutter_sdk_user.dart';
import 'package:naver_login_sdk/naver_login_sdk.dart';
import 'package:typie/env.dart';
import 'package:typie/firebase_options.dart';
import 'package:typie/styles/colors.dart';

Future<void> configureStaticServices() async {
  await SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
  SystemChrome.setSystemUIOverlayStyle(
    const SystemUiOverlayStyle(
      statusBarColor: AppColors.transparent,
      statusBarBrightness: Brightness.light,
      statusBarIconBrightness: Brightness.dark,
      systemNavigationBarColor: AppColors.transparent,
      systemNavigationBarDividerColor: AppColors.transparent,
      systemNavigationBarIconBrightness: Brightness.dark,
      systemStatusBarContrastEnforced: false,
      systemNavigationBarContrastEnforced: false,
    ),
  );

  await Firebase.initializeApp(options: DefaultFirebaseOptions.currentPlatform);

  KakaoSdk.init(nativeAppKey: Env.kakaoNativeAppKey);

  await NaverLoginSDK.initialize(
    urlScheme: 'typie',
    clientName: '타이피',
    clientId: Env.naverClientId,
    clientSecret: Env.naverClientSecret,
  );

  await Jiffy.setLocale('ko');
}
