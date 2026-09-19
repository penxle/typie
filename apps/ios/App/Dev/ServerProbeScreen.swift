import Core
import Design
import SwiftUI

struct ServerProbeScreen: View {
  @Environment(\.theme) private var theme
  private var colors: TColors { theme.colors }

  let probe: ServerProbe
  let authState: AuthStateStore
  @State private var results: [String: String] = [:]
  @State private var running: Set<String> = []

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 16) {
        row("API_URL host", probe.apiHost)
        row("AUTH_URL host", probe.authHost)
        row("X-Device-Id", probe.deviceID)
        row("AuthState", authStateLabel)
        row("userId", userId)
        action("query ServerProbe") { await probe.graphQL() }
        action("GET /authorize prompt=none") { await probe.authorize() }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(16)
    }
    .canvasBackground()
  }

  private var authStateLabel: String {
    switch authState.state {
    case .authenticated: "authenticated"
    case .unauthenticated: "unauthenticated"
    }
  }

  private var userId: String {
    if case .authenticated(let tokens) = authState.state { tokens.userId } else { "-" }
  }

  private func row(_ label: String, _ value: String) -> some View {
    VStack(alignment: .leading, spacing: 4) {
      TText(label, style: TTypography.caption, color: colors.textMuted)
      TText(value, style: TTypography.text, color: colors.textDefault)
    }
  }

  private func action(
    _ label: String, run: @escaping @Sendable () async -> String
  ) -> some View {
    VStack(alignment: .leading, spacing: 4) {
      TButton(label, variant: .secondary, loading: running.contains(label)) {
        running.insert(label)
        results[label] = await run()
        running.remove(label)
      }
      TText(results[label] ?? "-", style: TTypography.text, color: colors.textDefault)
    }
  }
}
