import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/providers/__generated__/me_query.req.gql.dart';
import 'package:typie/routers/app.dart';
import 'package:typie/screens/shell/marketing_consent_modal.dart';
import 'package:typie/services/auth.dart';

class MarketingConsentProvider extends HookWidget {
  const MarketingConsentProvider({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final client = useService<GraphQLClient>();
    final router = useService<AppRouter>();
    final authState = useValueListenable(auth);

    useEffect(() {
      if (authState is Authenticated) {
        unawaited(
          client.request(GMarketingConsentProvider_Me_QueryReq()).then((result) {
            final me = result.me;
            if (me != null && me.marketingConsentAskedAt == null && me.usage.totalCharacterCount >= 100) {
              final navigatorContext = router.navigatorKey.currentContext;
              if (navigatorContext != null && navigatorContext.mounted) {
                unawaited(navigatorContext.showModal(dismissible: false, child: MarketingConsentModal(client: client)));
              }
            }
          }),
        );
      }

      return null;
    }, [authState]);

    return const SizedBox.shrink();
  }
}
