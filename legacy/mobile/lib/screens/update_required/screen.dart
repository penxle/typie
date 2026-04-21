import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/svg_image.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:url_launcher/url_launcher.dart';

@RoutePage()
class UpdateRequiredScreen extends StatelessWidget {
  const UpdateRequiredScreen({
    required this.storeUrl,
    required this.currentVersion,
    required this.requiredVersion,
    super.key,
  });

  final String storeUrl;
  final String currentVersion;
  final String requiredVersion;

  @override
  Widget build(BuildContext context) {
    return Screen(
      safeArea: true,
      backgroundColor: context.colors.surfaceDefault,
      child: Column(
        children: [
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                const SvgImage('logos/full', height: 32),
                const Gap(24),
                const Text('업데이트가 필요해요', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600)),
                const Gap(4),
                const Text(
                  '새로운 버전이 출시되었어요.\n스토어에서 업데이트를 진행해주세요.',
                  style: TextStyle(fontSize: 14),
                  textAlign: TextAlign.center,
                ),
                const Gap(12),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(6)),
                  child: Column(
                    children: [
                      Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          SizedBox(
                            width: 60,
                            child: Text('현재 버전', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
                          ),
                          SizedBox(
                            width: 60,
                            child: Text(
                              currentVersion,
                              style: const TextStyle(fontSize: 13),
                              textAlign: TextAlign.right,
                            ),
                          ),
                        ],
                      ),
                      const Gap(4),
                      Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          SizedBox(
                            width: 60,
                            child: Text('필요 버전', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
                          ),
                          SizedBox(
                            width: 60,
                            child: Text(
                              requiredVersion,
                              style: const TextStyle(fontSize: 13),
                              textAlign: TextAlign.right,
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          Tappable(
            onTap: () async {
              final uri = Uri.parse('https://penxle.channel.io/home');
              await launchUrl(uri, mode: LaunchMode.externalApplication);
            },
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
              decoration: BoxDecoration(
                border: Border.all(color: context.colors.borderDefault),
                borderRadius: BorderRadius.circular(6),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(LucideLightIcons.headphones, size: 14, color: context.colors.textSubtle),
                  const Gap(6),
                  Text('고객센터', style: TextStyle(fontSize: 13, color: context.colors.textSubtle)),
                ],
              ),
            ),
          ),
          SafeArea(
            top: false,
            child: Padding(
              padding: const EdgeInsets.only(top: 16, bottom: 24),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 20),
                child: Tappable(
                  onTap: () async {
                    final uri = Uri.parse(storeUrl);
                    if (await canLaunchUrl(uri)) {
                      await launchUrl(uri, mode: LaunchMode.externalApplication);
                    }
                  },
                  child: Container(
                    height: 48,
                    decoration: BoxDecoration(
                      color: context.colors.surfaceInverse,
                      borderRadius: BorderRadius.circular(999),
                    ),
                    child: Center(
                      child: Text(
                        '업데이트하고 접속하기',
                        style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: context.colors.textInverse),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
