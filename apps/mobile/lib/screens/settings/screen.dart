import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:jiffy/jiffy.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/settings/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/settings/__generated__/update_marketing_consent_mutation.req.gql.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/services/theme.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/forms/switch.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:url_launcher/url_launcher.dart';

@RoutePage()
class SettingsScreen extends HookWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final pref = useService<Pref>();
    final theme = useService<AppTheme>();
    final mixpanel = useService<Mixpanel>();

    final packageInfoFuture = useMemoized(PackageInfo.fromPlatform);
    final packageInfo = useFuture(packageInfoFuture);

    final devModeTapCount = useState(0);

    return Screen(
      heading: const Heading(title: '설정'),
      child: GraphQLOperation(
        operation: GSettingsScreen_QueryReq(),
        builder: (context, client, data) {
          return SingleChildScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: Pad(all: 20, bottom: MediaQuery.paddingOf(context).bottom),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 24,
              children: [
                _Section(
                  title: '계정 설정',
                  children: [
                    _Item(
                      label: '이메일 변경',
                      onTap: () async {
                        await context.router.push(const UpdateEmailRoute());
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '프로필 변경',
                      onTap: () async {
                        await context.router.push(const UpdateProfileRoute());
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: (data.me?.hasPassword ?? false) ? '비밀번호 변경' : '비밀번호 설정',
                      onTap: () async {
                        await context.router.push(const UpdatePasswordRoute());
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '연결된 SNS 계정',
                      onTap: () async {
                        await context.router.push(const SocialAccountsRoute());
                      },
                    ),
                  ],
                ),
                _Section(
                  title: '화면 설정',
                  children: [
                    HookForm(
                      submitMode: HookFormSubmitMode.onChange,
                      onSubmit: (form) async {
                        final mode = form.data['themeMode'] as ThemeMode;

                        unawaited(
                          mixpanel.track(
                            'switch_theme',
                            properties: {'old': theme.mode.name, 'new': mode.name, 'via': 'settings'},
                          ),
                        );

                        theme.mode = mode;
                      },
                      builder: (context, form) {
                        return _Item(
                          label: '테마',
                          trailing: HookFormSelect<ThemeMode>(
                            name: 'themeMode',
                            initialValue: theme.mode,
                            items: const [
                              HookFormSelectItem(
                                label: '시스템 설정',
                                value: ThemeMode.system,
                                icon: LucideLightIcons.smartphone,
                              ),
                              HookFormSelectItem(label: '라이트', value: ThemeMode.light, icon: LucideLightIcons.sun),
                              HookFormSelectItem(label: '다크', value: ThemeMode.dark, icon: LucideLightIcons.moon),
                            ],
                          ),
                        );
                      },
                    ),
                  ],
                ),
                _Section(
                  title: '편집 경험 설정',
                  children: [
                    _Item(
                      label: '에디터 설정',
                      onTap: () async {
                        await context.router.push(const EditorSettingsRoute());
                      },
                    ),
                  ],
                ),
                _Section(
                  title: '이벤트 알림 설정',
                  children: [
                    HookForm(
                      submitMode: HookFormSubmitMode.onChange,
                      onSubmit: (form) async {
                        final marketingConsent = form.data['marketingConsent'] as bool;

                        await client.request(
                          GSettingsScreen_UpdateMarketingConsent_MutationReq(
                            (b) => b..vars.input.marketingConsent = marketingConsent,
                          ),
                        );

                        await mixpanel.track(
                          'update_marketing_consent',
                          properties: {'marketingConsent': marketingConsent},
                        );

                        if (context.mounted) {
                          await context.showModal(
                            child: AlertModal(
                              title: '타이피 마케팅 수신 동의',
                              message: '${Jiffy.now().yyyyMMddKorean}에 ${marketingConsent ? '동의' : '거부'}처리되었어요.',
                            ),
                          );
                        }
                      },
                      builder: (context, form) {
                        return _Item(
                          label: '이벤트 및 타이피 소식 받아보기',
                          trailing: HookFormSwitch(name: 'marketingConsent', initialValue: data.me!.marketingConsent),
                        );
                      },
                    ),
                  ],
                ),
                if (data.me!.subscription != null)
                  _Section(
                    title: '타이피 멤버십',
                    children: [
                      _Item(
                        label: '사이트 주소 변경',
                        onTap: () async {
                          await context.router.push(const UpdateSiteSlugRoute());
                        },
                      ),
                      const _Divider(),
                      _Item(
                        label: '타이피 커뮤니티 참여하기',
                        onTap: () async {
                          final url = Uri.parse('https://typie.link/community');
                          await launchUrl(url, mode: LaunchMode.externalApplication);
                        },
                      ),
                    ],
                  ),
                _Section(
                  title: '서비스 정보',
                  children: [
                    _Item(
                      label: '이용약관',
                      onTap: () async {
                        final url = Uri.parse('https://typie.co/legal/terms');
                        await launchUrl(url, mode: LaunchMode.inAppBrowserView);
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '개인정보처리방침',
                      onTap: () async {
                        final url = Uri.parse('https://typie.co/legal/privacy');
                        await launchUrl(url, mode: LaunchMode.inAppBrowserView);
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '사업자 정보',
                      onTap: () async {
                        final url = Uri.parse('https://www.ftc.go.kr/bizCommPop.do?wrkr_no=6108803078');
                        await launchUrl(url, mode: LaunchMode.inAppBrowserView);
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '오픈소스 라이센스',
                      onTap: () async {
                        await context.router.push(const OssLicensesRoute());
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '버전 정보',
                      trailing: packageInfo.hasData
                          ? Text(
                              '${packageInfo.data!.version} (${kDebugMode ? 'dev' : packageInfo.data!.buildNumber})',
                              style: const TextStyle(fontSize: 16),
                            )
                          : const SizedBox.square(dimension: 16, child: CircularProgressIndicator()),
                      onTap: () {
                        if (pref.devMode) {
                          context.toast(ToastType.success, '이미 개발자입니다.');
                          return;
                        }

                        devModeTapCount.value += 1;

                        if (devModeTapCount.value >= 7) {
                          pref.devMode = true;
                          context.toast(ToastType.success, '개발자가 되셨습니다.');
                          return;
                        }

                        if (devModeTapCount.value >= 4) {
                          context.toast(ToastType.success, '개발자가 되기까지 ${7 - devModeTapCount.value}번...');
                          return;
                        }
                      },
                    ),
                  ],
                ),
                if (pref.devMode)
                  _Section(
                    title: '개발자',
                    children: [
                      _Item(
                        label: '개발자 모드',
                        trailing: HookForm(
                          submitMode: HookFormSubmitMode.onChange,
                          onSubmit: (form) async {
                            pref.devMode = form.data['devMode'] as bool;
                          },
                          builder: (context, form) {
                            return HookFormSwitch(name: 'devMode', initialValue: pref.devMode);
                          },
                        ),
                      ),
                    ],
                  ),
                _Section(
                  title: '기타',
                  children: [
                    _Item(
                      label: '로그아웃',
                      onTap: () async {
                        await context.showModal(
                          child: ConfirmModal(
                            title: '로그아웃',
                            message: '정말 로그아웃하시겠어요?',
                            confirmText: '로그아웃',
                            onConfirm: () async {
                              unawaited(mixpanel.track('logout', properties: {'via': 'profile'}));
                              await auth.logout();
                            },
                          ),
                        );
                      },
                    ),
                    const _Divider(),
                    _Item(
                      label: '회원 탈퇴',
                      onTap: () async {
                        await context.router.push(const DeleteUserRoute());
                      },
                    ),
                  ],
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 8,
      children: [
        Text(
          title,
          style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textFaint),
        ),
        Container(
          decoration: BoxDecoration(
            border: Border.all(color: context.colors.borderStrong),
            borderRadius: BorderRadius.circular(8),
            color: context.colors.surfaceDefault,
          ),
          child: Column(crossAxisAlignment: CrossAxisAlignment.stretch, children: children),
        ),
      ],
    );
  }
}

class _Divider extends StatelessWidget {
  const _Divider();

  @override
  Widget build(BuildContext context) {
    return HorizontalDivider(color: context.colors.borderDefault);
  }
}

class _Item extends StatelessWidget {
  const _Item({required this.label, this.trailing, this.onTap});

  final void Function()? onTap;
  final String label;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final child = Row(
      children: [
        Expanded(child: Text(label, style: const TextStyle(fontSize: 16))),
        if (trailing == null) const Icon(LucideLightIcons.chevron_right, size: 16) else trailing!,
      ],
    );

    if (onTap == null) {
      return Padding(padding: const Pad(all: 16), child: child);
    } else {
      return Tappable(onTap: onTap!, padding: const Pad(all: 16), child: child);
    }
  }
}
