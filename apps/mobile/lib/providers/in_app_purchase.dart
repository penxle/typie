import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:in_app_purchase/in_app_purchase.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/logger.dart';
import 'package:typie/providers/__generated__/subscribe_or_change_plan_with_in_app_purchase_mutation.req.gql.dart';
import 'package:typie/services/auth.dart';

class InAppPurchaseProvider extends HookWidget {
  const InAppPurchaseProvider({super.key});

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final client = useService<GraphQLClient>();
    final authState = useValueListenable(auth);

    useEffect(() {
      if (authState is! Authenticated) {
        return null;
      }

      final subscription = InAppPurchase.instance.purchaseStream.listen((purchaseDetailsList) async {
        for (final purchaseDetails in purchaseDetailsList) {
          await _handlePurchase(client, purchaseDetails);
        }
      });

      return subscription.cancel;
    }, [authState]);

    return const SizedBox.shrink();
  }
}

Future<void> _handlePurchase(GraphQLClient client, PurchaseDetails purchaseDetails) async {
  if (purchaseDetails.error != null) {
    await Sentry.captureException(purchaseDetails.error);
    log.e('InAppPurchaseProvider', error: purchaseDetails.error);
  }

  try {
    await client.request(
      GInAppPurchaseProvider_SubscribeOrChangePlanWithInAppPurchase_MutationReq(
        (b) => b
          ..vars.input.store = Platform.isIOS ? GInAppPurchaseStore.APP_STORE : GInAppPurchaseStore.GOOGLE_PLAY
          ..vars.input.data = Platform.isIOS
              ? purchaseDetails.purchaseID
              : purchaseDetails.verificationData.serverVerificationData,
      ),
    );
  } catch (err) {
    await Sentry.captureException(err);
    log.e('InAppPurchaseProvider', error: err);
  } finally {
    if (purchaseDetails.pendingCompletePurchase) {
      await InAppPurchase.instance.completePurchase(purchaseDetails);
    }
  }
}
