import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/social_accounts/__generated__/screen_query.req.gql.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/settings_screen.dart';
import 'package:typie/widgets/svg_image.dart';

@RoutePage()
class SocialAccountsScreen extends HookWidget {
  const SocialAccountsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final scrollController = useScrollController();

    return GraphQLOperation(
      operation: GSocialAccountsScreen_QueryReq(),
      builder: (context, client, data) {
        return SettingsOverlayScreen(
          title: '연결된 SNS 계정',
          scrollController: scrollController,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Gap(settingsSectionGap),
              const SettingsSectionLabel(text: '연결된 계정'),
              SettingsSectionCard(
                clipBehavior: Clip.antiAlias,
                child: data.me!.singleSignOns.isEmpty
                    ? SizedBox(
                        height: 220,
                        child: Center(
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            spacing: 12,
                            children: [
                              Icon(LucideLightIcons.user_x, size: 48, color: context.colors.textFaint),
                              Text('연결된 SNS 계정이 없어요', style: TextStyle(fontSize: 16, color: context.colors.textFaint)),
                            ],
                          ),
                        ),
                      )
                    : Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          for (final sso in data.me!.singleSignOns)
                            Column(
                              children: [
                                _SnsItem(provider: sso.provider.name, email: sso.email),
                                if (sso != data.me!.singleSignOns.last)
                                  HorizontalDivider(color: context.colors.borderSubtle),
                              ],
                            ),
                        ],
                      ),
              ),
            ],
          ),
        );
      },
    );
  }
}

class _SnsItem extends StatelessWidget {
  const _SnsItem({required this.provider, required this.email});

  final String provider;
  final String email;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const Pad(all: 16),
      child: Row(
        spacing: 12,
        children: [
          _ProviderIcon(provider: provider),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 2,
              children: [
                Text(_getProviderName(provider), style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w500)),
                Text(email, style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              ],
            ),
          ),
        ],
      ),
    );
  }

  String _getProviderName(String provider) {
    switch (provider) {
      case 'GOOGLE':
        return 'Google';
      case 'NAVER':
        return 'Naver';
      case 'KAKAO':
        return 'Kakao';
      default:
        return provider;
    }
  }
}

class _ProviderIcon extends StatelessWidget {
  const _ProviderIcon({required this.provider});

  final String provider;

  @override
  Widget build(BuildContext context) {
    switch (provider) {
      case 'GOOGLE':
        return const SizedBox(width: 28, height: 28, child: SvgImage('brands/google', width: 28, height: 28));
      case 'NAVER':
        return Container(
          width: 28,
          height: 28,
          decoration: BoxDecoration(borderRadius: BorderRadius.circular(6), color: const Color(0xFF03C75A)),
          child: const Center(child: SvgImage('brands/naver', width: 16, height: 16, color: Colors.white)),
        );
      case 'KAKAO':
        return Container(
          width: 28,
          height: 28,
          decoration: BoxDecoration(borderRadius: BorderRadius.circular(6), color: const Color(0xFFFEE500)),
          child: const Center(child: SvgImage('brands/kakao', width: 20, height: 20)),
        );
      default:
        return Container(
          width: 28,
          height: 28,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(6),
            color: context.colors.surfaceMuted,
            border: Border.all(color: context.colors.borderDefault),
          ),
          child: Icon(LucideLightIcons.user, size: 20, color: context.colors.textFaint),
        );
    }
  }
}
