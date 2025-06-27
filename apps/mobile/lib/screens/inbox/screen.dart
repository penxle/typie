import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class InboxScreen extends StatelessWidget {
  const InboxScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      heading: const Heading(title: '알림', titleIcon: LucideLightIcons.inbox),
      child: Center(
        child: Text('아직 알림이 없어요', style: TextStyle(fontSize: 15, color: context.colors.textFaint)),
      ),
    );
  }
}
