#if canImport(UIKit)

  import Design
  import UIKit

  extension MainTab {
    var label: String {
      switch self {
      case .home: "홈"
      case .studio: "스페이스"
      case .notes: "노트"
      }
    }

    var image: UIImage {
      let name =
        switch self {
        case .home: LucideIcon.house
        case .studio: LucideIcon.folderOpen
        case .notes: LucideIcon.stickyNote
        }
      return UIImage(named: name.assetName, in: TDesignBundle.bundle, with: nil)!
    }

    var selectedImage: UIImage {
      let name =
        switch self {
        case .home: TypieIcon.houseFilled
        case .studio: TypieIcon.folderOpenFilled
        case .notes: TypieIcon.stickyNoteFilled
        }
      return UIImage(named: name.assetName, in: TDesignBundle.bundle, with: nil)!
    }
  }

#endif
