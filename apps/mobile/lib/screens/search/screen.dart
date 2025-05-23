import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class SearchScreen extends StatelessWidget {
  const SearchScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const Screen(child: Center(child: Text('search')));
  }
}
