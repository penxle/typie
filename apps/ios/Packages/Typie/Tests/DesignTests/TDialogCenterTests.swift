import Testing

@testable import Design

@MainActor
@Suite struct TDialogCenterTests {
  nonisolated private static let failureTitle = "로그인할 수 없어요"
  nonisolated private static let failureMessage = "오류가 발생했어요. 잠시 후 다시 시도해주세요."
  nonisolated private static let confirmText = "확인"
  nonisolated private static let retryTitle = "문제가 발생했어요"
  nonisolated private static let retryMessage = "잠시 후 다시 시도해주세요."
  nonisolated private static let retryText = "다시 시도"

  private static func failure() -> TDialogItem {
    TDialogItem(title: failureTitle, message: failureMessage, confirmText: confirmText)
  }

  private static func retry() -> TDialogItem {
    TDialogItem(title: retryTitle, message: retryMessage, confirmText: retryText)
  }

  @Test func startsEmpty() {
    #expect(TDialogCenter().current == nil)
  }

  @Test func presentPublishesItem() {
    let center = TDialogCenter()
    let item = Self.failure()

    center.present(item)

    #expect(center.current == item)
  }

  @Test func dismissClearsWithoutInvokingCallback() {
    let center = TDialogCenter()
    let recorder = DismissRecorder()

    center.present(Self.failure()) { [weak center] in
      recorder.record(cleared: center?.current == nil)
    }
    center.dismiss()

    #expect(center.current == nil)
    #expect(recorder.count == 0)
  }

  @Test func confirmClearsBeforeInvokingCallbackOnce() {
    let center = TDialogCenter()
    let recorder = DismissRecorder()

    center.present(Self.failure()) { [weak center] in
      recorder.record(cleared: center?.current == nil)
    }
    center.confirm()

    #expect(center.current == nil)
    #expect(recorder.count == 1)
    #expect(recorder.clearedWhenCalled == true)

    center.confirm()

    #expect(recorder.count == 1)
  }

  @Test func repeatedPresentKeepsLastItemAndCallback() {
    let center = TDialogCenter()
    let first = DismissRecorder()
    let second = DismissRecorder()
    let latest = Self.retry()

    center.present(Self.failure()) { first.record(cleared: true) }
    center.present(latest) { second.record(cleared: true) }

    #expect(center.current == latest)

    center.confirm()

    #expect(first.count == 0)
    #expect(second.count == 1)
  }
}

@MainActor
private final class DismissRecorder {
  private(set) var count = 0
  private(set) var clearedWhenCalled: Bool?

  func record(cleared: Bool) {
    count += 1
    clearedWhenCalled = cleared
  }
}
