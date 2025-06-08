import 'dart:async';
import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:google_sign_in/google_sign_in.dart';
import 'package:kakao_flutter_sdk_user/kakao_flutter_sdk_user.dart';
import 'package:naver_login_sdk/naver_login_sdk.dart';
import 'package:sign_in_with_apple/sign_in_with_apple.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/login/__generated__/authorize_single_sign_on_mutation.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/svg_image.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class LoginScreen extends HookWidget {
  const LoginScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final googleSignIn = useService<GoogleSignIn>();

    final login = useCallback((GSingleSignOnProvider provider, Map<String, dynamic> params) async {
      await context.runWithLoader(() async {
        await client.request(
          GLoginScreen_AuthorizeSingleSignOn_MutationReq(
            (b) => b
              ..vars.input.provider = provider
              ..vars.input.params = JsonObject(params),
          ),
        );
      });
    });

    return Screen(
      safeArea: true,
      backgroundColor: AppColors.white,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                SvgImage('logos/full', height: 48),
                Gap(24),
                Text('온전히 생각을 담아내는', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w400)),
                Gap(4),
                Text('나만의 글쓰기 공간', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700)),
              ],
            ),
          ),
          Padding(
            padding: const Pad(horizontal: 20),
            child: Column(
              spacing: 8,
              children: [
                _Button(
                  text: '구글로 시작하기',
                  icon: const SvgImage('brands/google', width: 20),
                  borderColor: AppColors.gray_200,
                  foregroundColor: AppColors.gray_950,
                  backgroundColor: AppColors.white,
                  onTap: () async {
                    await googleSignIn.signOut();
                    final result = await googleSignIn.signIn();

                    if (result != null) {
                      await login(GSingleSignOnProvider.GOOGLE, {'code': result.serverAuthCode});
                    }
                  },
                ),
                _Button(
                  text: '카카오로 시작하기',
                  icon: const SvgImage('brands/kakao', width: 20, color: AppColors.black),
                  foregroundColor: AppColors.gray_950,
                  backgroundColor: const Color(0xFFFEE500),
                  onTap: () async {
                    if (!await isKakaoTalkInstalled()) {
                      if (context.mounted) {
                        context.toast(ToastType.error, '카카오톡을 먼저 설치해주세요.');
                      }

                      return;
                    }

                    try {
                      await UserApi.instance.logout();
                    } catch (_) {
                      // pass
                    }

                    final result = await UserApi.instance.loginWithKakaoTalk();
                    await login(GSingleSignOnProvider.KAKAO, {'access_token': result.accessToken});
                  },
                ),
                _Button(
                  text: '네이버로 시작하기',
                  icon: const SvgImage('brands/naver', width: 20, color: AppColors.white),
                  foregroundColor: AppColors.white,
                  backgroundColor: const Color(0xFF03C75A),
                  onTap: () async {
                    final completer = Completer<bool>();

                    await NaverLoginSDK.logout();
                    await NaverLoginSDK.authenticate(
                      callback: OAuthLoginCallback(
                        onSuccess: () {
                          completer.complete(true);
                        },
                        onError: (code, message) {
                          if (code == 2) {
                            completer.complete(false);
                          } else {
                            completer.completeError(Exception('[$code] $message'));
                          }
                        },
                        onFailure: (code, message) {
                          completer.completeError(Exception('[$code] $message'));
                        },
                      ),
                    );

                    if (await completer.future) {
                      final accessToken = await NaverLoginSDK.getAccessToken();
                      await login(GSingleSignOnProvider.NAVER, {'access_token': accessToken});
                    }
                  },
                ),
                if (Platform.isIOS)
                  _Button(
                    text: '애플로 시작하기',
                    icon: const SvgImage('brands/apple', width: 20, color: AppColors.white),
                    foregroundColor: AppColors.white,
                    backgroundColor: AppColors.gray_950,
                    onTap: () async {
                      final result = await SignInWithApple.getAppleIDCredential(
                        scopes: [AppleIDAuthorizationScopes.email],
                      );

                      await login(GSingleSignOnProvider.APPLE, {'code': result.authorizationCode});
                    },
                  ),
                Tappable(
                  padding: const Pad(horizontal: 24, vertical: 8),
                  onTap: () async {
                    await context.router.push(const LoginWithEmailRoute());
                  },
                  child: const Text('이메일로 가입하셨나요?', style: TextStyle(fontSize: 14, color: AppColors.gray_700)),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _Button extends StatelessWidget {
  const _Button({
    required this.text,
    required this.foregroundColor,
    required this.backgroundColor,
    required this.onTap,
    this.borderColor,
    this.icon,
  });

  final Widget? icon;
  final String text;
  final Color foregroundColor;
  final Color backgroundColor;
  final Color? borderColor;
  final Future<void> Function() onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: () async {
        try {
          await onTap();
        } catch (_) {
          if (context.mounted) {
            context.toast(ToastType.error, '로그인에 실패했어요. 다시 시도해주세요.');
          }
        }
      },
      child: Container(
        height: 48,
        decoration: BoxDecoration(
          border: Border.all(color: borderColor ?? backgroundColor),
          borderRadius: BorderRadius.circular(999),
          color: backgroundColor,
        ),
        child: Stack(
          children: [
            if (icon != null) Positioned(top: 0, bottom: 0, left: 24, child: icon!),
            Center(
              child: Text(
                text,
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: foregroundColor),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
