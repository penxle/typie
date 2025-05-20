import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class PostsScreen extends HookWidget {
  const PostsScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return const Screen(child: Center(child: Text('posts')));
  }
}
