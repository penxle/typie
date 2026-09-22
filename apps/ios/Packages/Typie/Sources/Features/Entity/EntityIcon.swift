import Design
import SwiftUI

enum EntityKind: Equatable, Sendable {
  case document
  case folder
}

struct EntityIconSpec: Equatable, Sendable {
  let kind: EntityKind
  let name: String
  let color: String

  init(kind: EntityKind, name: String, color: String) {
    self.kind = kind
    self.name = name
    self.color = color
  }
}

struct EntityIconAppearance: Equatable {
  let icon: TIconName
  let tint: Color
}

enum EntityIcon {
  static func appearance(_ spec: EntityIconSpec, colors: TColors) -> EntityIconAppearance {
    let fallback = spec.kind == .folder ? LucideIcon.folder : LucideIcon.file
    let icon = names[spec.name.trimmingCharacters(in: .whitespaces)] ?? fallback
    let tint = tint(spec.color, colors: colors) ?? colors.textMuted
    return EntityIconAppearance(icon: icon, tint: tint)
  }

  static func tint(_ color: String, colors: TColors) -> Color? {
    switch color.trimmingCharacters(in: .whitespaces) {
    case "gray": colors.paletteGray
    case "red": colors.paletteRed
    case "orange": colors.paletteOrange
    case "yellow": colors.paletteYellow
    case "green": colors.paletteGreen
    case "blue": colors.paletteBlue
    case "purple": colors.palettePurple
    default: nil
    }
  }

  static let names: [String: TIconName] = [
    "file": LucideIcon.file, "file-text": LucideIcon.fileText, "notebook": LucideIcon.notebook,
    "book": LucideIcon.book, "book-open": LucideIcon.bookOpen, "folder": LucideIcon.folder,
    "archive": LucideIcon.archive, "inbox": LucideIcon.inbox, "clipboard": LucideIcon.clipboard,
    "layers": LucideIcon.layers, "layout-template": LucideIcon.layoutTemplate,
    "table": LucideIcon.table,
    "list": LucideIcon.list, "palette": LucideIcon.palette, "pen-tool": LucideIcon.penTool,
    "brush": LucideIcon.brush, "feather": LucideIcon.feather, "wand": LucideIcon.wand,
    "sticker": LucideIcon.sticker, "lightbulb": LucideIcon.lightbulb,
    "sparkles": LucideIcon.sparkles,
    "rocket": LucideIcon.rocket, "zap": LucideIcon.zap, "bolt": LucideIcon.bolt,
    "flame": LucideIcon.flame, "star": LucideIcon.star, "heart": LucideIcon.heart,
    "bookmark": LucideIcon.bookmark, "flag": LucideIcon.flag, "tag": LucideIcon.tag,
    "pin": LucideIcon.pin, "circle-check": LucideIcon.circleCheck, "target": LucideIcon.target,
    "trophy": LucideIcon.trophy, "award": LucideIcon.award, "crown": LucideIcon.crown,
    "image": LucideIcon.image, "video": LucideIcon.video, "camera": LucideIcon.camera,
    "music": LucideIcon.music, "mic": LucideIcon.mic, "headphones": LucideIcon.headphones,
    "speaker": LucideIcon.speaker, "radio": LucideIcon.radio,
    "graduation-cap": LucideIcon.graduationCap,
    "glasses": LucideIcon.glasses, "languages": LucideIcon.languages,
    "flask-conical": LucideIcon.flaskConical,
    "search": LucideIcon.search, "eye": LucideIcon.eye, "sun": LucideIcon.sun,
    "moon": LucideIcon.moon, "leaf": LucideIcon.leaf, "trees": LucideIcon.trees,
    "mountain": LucideIcon.mountain, "droplet": LucideIcon.droplet, "umbrella": LucideIcon.umbrella,
    "cloud": LucideIcon.cloud, "thermometer": LucideIcon.thermometer, "coffee": LucideIcon.coffee,
    "smile": LucideIcon.smile, "gift": LucideIcon.gift, "cake": LucideIcon.cake,
    "diamond": LucideIcon.diamond, "gem": LucideIcon.gem, "puzzle": LucideIcon.puzzle,
    "dices": LucideIcon.dices, "sword": LucideIcon.sword, "infinity": LucideIcon.infinity,
    "paperclip": LucideIcon.paperclip, "key": LucideIcon.key, "lock": LucideIcon.lock,
    "mail": LucideIcon.mail, "send": LucideIcon.send, "message-square": LucideIcon.messageSquare,
    "megaphone": LucideIcon.megaphone, "bell": LucideIcon.bell, "phone": LucideIcon.phone,
    "at-sign": LucideIcon.atSign, "hash": LucideIcon.hash, "users": LucideIcon.users,
    "handshake": LucideIcon.handshake, "briefcase": LucideIcon.briefcase,
    "calendar": LucideIcon.calendar,
    "clock": LucideIcon.clock, "alarm-clock": LucideIcon.alarmClock, "timer": LucideIcon.timer,
    "home": LucideIcon.house, "building": LucideIcon.building, "landmark": LucideIcon.landmark,
    "map": LucideIcon.map, "map-pin": LucideIcon.mapPin, "compass": LucideIcon.compass,
    "navigation": LucideIcon.navigation, "plane": LucideIcon.plane, "truck": LucideIcon.truck,
    "globe": LucideIcon.globe, "cog": LucideIcon.cog, "wrench": LucideIcon.wrench,
    "hammer": LucideIcon.hammer, "scissors": LucideIcon.scissors, "ruler": LucideIcon.ruler,
    "shield": LucideIcon.shield, "fingerprint": LucideIcon.fingerprintPattern,
    "code": LucideIcon.code,
    "terminal": LucideIcon.terminal, "database": LucideIcon.database, "server": LucideIcon.server,
    "cpu": LucideIcon.cpu, "plug": LucideIcon.plug, "bug": LucideIcon.bug,
    "link": LucideIcon.link, "monitor": LucideIcon.monitor, "smartphone": LucideIcon.smartphone,
    "tv": LucideIcon.tv, "battery": LucideIcon.battery, "download": LucideIcon.download,
    "wallet": LucideIcon.wallet, "credit-card": LucideIcon.creditCard,
    "banknote": LucideIcon.banknote,
    "dollar-sign": LucideIcon.dollarSign, "shopping-cart": LucideIcon.shoppingCart,
    "ticket": LucideIcon.ticket,
    "bar-chart-2": LucideIcon.barChartBig, "pie-chart": LucideIcon.pieChart,
    "scale": LucideIcon.scale,
    "package": LucideIcon.package2, "box": LucideIcon.box, "ear": LucideIcon.ear,
  ]
}
