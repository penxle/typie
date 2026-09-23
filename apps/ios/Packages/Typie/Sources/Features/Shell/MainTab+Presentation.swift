#if canImport(UIKit)

  import Design
  import UIKit

  extension MainTab {
    var label: String {
      switch self {
      case .home: "홈"
      case .notes: "노트"
      case .prism: "프리즘"
      case .square: "스퀘어"
      }
    }

    var image: UIImage {
      let name =
        switch self {
        case .home: TypieIcon.home
        case .notes: TypieIcon.note
        case .prism: TypieIcon.prism
        case .square: TypieIcon.square
        }
      return UIImage(named: name.assetName, in: TDesignBundle.bundle, with: nil)!
    }

    var selectedImage: UIImage {
      let name =
        switch self {
        case .home: TypieIcon.homeFilled
        case .notes: TypieIcon.noteFilled
        case .prism: TypieIcon.prismFilled
        case .square: TypieIcon.squareFilled
        }
      return UIImage(named: name.assetName, in: TDesignBundle.bundle, with: nil)!
    }
  }

#endif
