import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:ferry/ferry.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/screen.dart';

class GraphQLOperation<TData, TVars> extends HookWidget {
  const GraphQLOperation({required this.operation, required this.builder, this.onLoaded, super.key});

  final OperationRequest<TData, TVars> operation;
  final Widget Function(BuildContext context, GraphQLClient client, TData data) builder;
  final void Function(TData data)? onLoaded;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final stream = useMemoized(() => client.raw.request(operation).distinct(), [operation]);
    final snapshot = useStream(stream);
    final loaded = useRef(false);

    final controller = useAnimationController(duration: const Duration(milliseconds: 200));
    final tweenedOpacity = useMemoized(() {
      final curve = CurvedAnimation(parent: controller, curve: Curves.ease);
      return Tween<double>(begin: 0, end: 1).animate(curve);
    }, [controller]);

    useEffect(() {
      final data = snapshot.data?.data;
      if (data != null && !loaded.value) {
        loaded.value = true;
        controller.forward();
        onLoaded?.call(data);
      }

      return null;
    }, [snapshot.data]);

    final data = snapshot.data?.data;
    if (data == null) {
      return const Screen(child: SizedBox.expand());
    }

    return Box(
      color: AppColors.white,
      child: FadeTransition(opacity: tweenedOpacity, child: builder(context, client, data)),
    );
  }
}
