import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class SearchScreen extends StatelessWidget {
  const SearchScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const Screen(
      heading: Heading(title: '검색', titleIcon: LucideLightIcons.search),
      padding: Pad(all: 20),
      child: Placeholder(strokeWidth: 1, color: AppColors.gray_950),
    );
  }
}
