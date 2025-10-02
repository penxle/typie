import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';

void useRouteResumed(BuildContext context, VoidCallback onResumed, {int? tabIndex}) {
  final wasInactive = useRef(false);
  final route = ModalRoute.of(context);
  final isCurrent = route?.isCurrent ?? false;

  useEffect(() {
    if (isCurrent && wasInactive.value) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        onResumed();
      });
      wasInactive.value = false;
    } else if (!isCurrent && !wasInactive.value) {
      wasInactive.value = true;
    }
    return null;
  }, [isCurrent, onResumed]);

  useEffect(() {
    if (tabIndex == null) {
      return null;
    }

    final tabsRouter = AutoTabsRouter.of(context);

    void onTabChange() {
      if (tabsRouter.activeIndex == tabIndex) {
        onResumed();
      }
    }

    tabsRouter.addListener(onTabChange);
    return () => tabsRouter.removeListener(onTabChange);
  }, [tabIndex, onResumed]);
}
