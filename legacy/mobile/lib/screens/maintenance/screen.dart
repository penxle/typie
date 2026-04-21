import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:jiffy/jiffy.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/svg_image.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:url_launcher/url_launcher.dart';

@RoutePage()
class MaintenanceScreen extends StatelessWidget {
  const MaintenanceScreen({required this.title, required this.message, this.until, super.key});

  final String title;
  final String message;
  final DateTime? until;

  @override
  Widget build(BuildContext context) {
    return Screen(
      safeArea: true,
      backgroundColor: context.colors.surfaceDefault,
      child: Column(
        children: [
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  const SvgImage('logos/full', height: 32),
                  const Gap(24),
                  Text(title, style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600)),
                  const Gap(4),
                  Text(
                    message.replaceAll(r'\n', '\n'),
                    style: const TextStyle(fontSize: 14),
                    textAlign: TextAlign.center,
                  ),
                  if (until != null) ...[
                    const Gap(12),
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                      decoration: BoxDecoration(
                        color: context.colors.surfaceMuted,
                        borderRadius: BorderRadius.circular(6),
                      ),
                      child: Text(
                        '예상 종료: ${Jiffy.parseFromDateTime(until!.toLocal()).format(pattern: 'M월 d일 HH시 mm분')}',
                        style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                      ),
                    ),
                  ],
                ],
              ),
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
          const SafeArea(top: false, child: SizedBox(height: 24)),
        ],
      ),
    );
  }
}
