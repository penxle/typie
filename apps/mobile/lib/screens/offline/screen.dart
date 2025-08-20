import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class OfflineScreen extends HookWidget {
  const OfflineScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();

    return Material(
      color: context.colors.surfaceDefault,
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(LucideLightIcons.cloud_off, size: 48, color: context.colors.textFaint),
          const Gap(16),
          const Text('앗! 문제가 발생했어요', style: TextStyle(fontSize: 16)),
          Text('네트워크 연결을 확인하거나 잠시 후 다시 시도해주세요.', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
          const Gap(16),
          Tappable(
            onTap: () async {
              await auth.retry();
            },
            child: Container(
              decoration: BoxDecoration(
                border: Border.all(color: context.colors.borderStrong),
                borderRadius: BorderRadius.circular(8),
              ),
              padding: const Pad(horizontal: 16, vertical: 8),
              child: const Text('다시 시도', style: TextStyle(fontSize: 15)),
            ),
          ),
        ],
      ),
    );
  }
}
