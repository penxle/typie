package co.typie.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import co.typie.icons.Lucide
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppColors
import co.typie.ui.theme.AppTheme

data class EntityIconAppearance(val icon: IconData, val tint: Color)

// Keep this in sync with the website picker so mobile renders the same entity icon names.
private val entityIconMap =
  mapOf(
    "file" to Lucide.File,
    "file-text" to Lucide.FileText,
    "notebook" to Lucide.Notebook,
    "book" to Lucide.Book,
    "book-open" to Lucide.BookOpen,
    "folder" to Lucide.Folder,
    "archive" to Lucide.Archive,
    "inbox" to Lucide.Inbox,
    "clipboard" to Lucide.Clipboard,
    "layers" to Lucide.Layers,
    "layout-template" to Lucide.LayoutTemplate,
    "table" to Lucide.Table,
    "list" to Lucide.List,
    "palette" to Lucide.Palette,
    "pen-tool" to Lucide.PenTool,
    "brush" to Lucide.Brush,
    "feather" to Lucide.Feather,
    "wand" to Lucide.Wand,
    "sticker" to Lucide.Sticker,
    "lightbulb" to Lucide.Lightbulb,
    "sparkles" to Lucide.Sparkles,
    "rocket" to Lucide.Rocket,
    "zap" to Lucide.Zap,
    "bolt" to Lucide.Bolt,
    "flame" to Lucide.Flame,
    "star" to Lucide.Star,
    "heart" to Lucide.Heart,
    "bookmark" to Lucide.Bookmark,
    "flag" to Lucide.Flag,
    "tag" to Lucide.Tag,
    "pin" to Lucide.Pin,
    "circle-check" to Lucide.CircleCheck,
    "target" to Lucide.Target,
    "trophy" to Lucide.Trophy,
    "award" to Lucide.Award,
    "crown" to Lucide.Crown,
    "image" to Lucide.Image,
    "video" to Lucide.Video,
    "camera" to Lucide.Camera,
    "music" to Lucide.Music,
    "mic" to Lucide.Mic,
    "headphones" to Lucide.Headphones,
    "speaker" to Lucide.Speaker,
    "radio" to Lucide.Radio,
    "graduation-cap" to Lucide.GraduationCap,
    "glasses" to Lucide.Glasses,
    "languages" to Lucide.Languages,
    "flask-conical" to Lucide.FlaskConical,
    "search" to Lucide.Search,
    "eye" to Lucide.Eye,
    "sun" to Lucide.Sun,
    "moon" to Lucide.Moon,
    "leaf" to Lucide.Leaf,
    "trees" to Lucide.Trees,
    "mountain" to Lucide.Mountain,
    "droplet" to Lucide.Droplet,
    "umbrella" to Lucide.Umbrella,
    "cloud" to Lucide.Cloud,
    "thermometer" to Lucide.Thermometer,
    "coffee" to Lucide.Coffee,
    "smile" to Lucide.Smile,
    "gift" to Lucide.Gift,
    "cake" to Lucide.Cake,
    "diamond" to Lucide.Diamond,
    "gem" to Lucide.Gem,
    "puzzle" to Lucide.Puzzle,
    "dices" to Lucide.Dices,
    "sword" to Lucide.Sword,
    "infinity" to Lucide.Infinity,
    "paperclip" to Lucide.Paperclip,
    "key" to Lucide.Key,
    "lock" to Lucide.Lock,
    "mail" to Lucide.Mail,
    "send" to Lucide.Send,
    "message-square" to Lucide.MessageSquare,
    "megaphone" to Lucide.Megaphone,
    "bell" to Lucide.Bell,
    "phone" to Lucide.Phone,
    "at-sign" to Lucide.AtSign,
    "hash" to Lucide.Hash,
    "users" to Lucide.Users,
    "handshake" to Lucide.Handshake,
    "briefcase" to Lucide.Briefcase,
    "calendar" to Lucide.Calendar,
    "clock" to Lucide.Clock,
    "alarm-clock" to Lucide.AlarmClock,
    "timer" to Lucide.Timer,
    "home" to Lucide.House,
    "building" to Lucide.Building,
    "landmark" to Lucide.Landmark,
    "map" to Lucide.Map,
    "map-pin" to Lucide.MapPin,
    "compass" to Lucide.Compass,
    "navigation" to Lucide.Navigation,
    "plane" to Lucide.Plane,
    "truck" to Lucide.Truck,
    "globe" to Lucide.Globe,
    "cog" to Lucide.Cog,
    "wrench" to Lucide.Wrench,
    "hammer" to Lucide.Hammer,
    "scissors" to Lucide.Scissors,
    "ruler" to Lucide.Ruler,
    "shield" to Lucide.Shield,
    "fingerprint" to Lucide.FingerprintPattern,
    "code" to Lucide.Code,
    "terminal" to Lucide.Terminal,
    "database" to Lucide.Database,
    "server" to Lucide.Server,
    "cpu" to Lucide.Cpu,
    "plug" to Lucide.Plug,
    "bug" to Lucide.Bug,
    "link" to Lucide.Link,
    "monitor" to Lucide.Monitor,
    "smartphone" to Lucide.Smartphone,
    "tv" to Lucide.Tv,
    "battery" to Lucide.Battery,
    "download" to Lucide.Download,
    "wallet" to Lucide.Wallet,
    "credit-card" to Lucide.CreditCard,
    "banknote" to Lucide.Banknote,
    "dollar-sign" to Lucide.DollarSign,
    "shopping-cart" to Lucide.ShoppingCart,
    "ticket" to Lucide.Ticket,
    "bar-chart-2" to Lucide.BarChartBig,
    "pie-chart" to Lucide.PieChart,
    "scale" to Lucide.Scale,
    "package" to Lucide.Package2,
    "box" to Lucide.Box,
    "ear" to Lucide.Ear,
  )

data class EntityIconOption(val name: String, val icon: IconData)

data class EntityIconColorOption(val label: String, val value: String, val color: Color)

val entityIcons = entityIconMap.map { (name, icon) -> EntityIconOption(name = name, icon = icon) }

@Composable
internal fun rememberEntityIconColorOptions(): List<EntityIconColorOption> {
  val palette = AppTheme.colors.palette
  return listOf(
    EntityIconColorOption("그레이", "gray", palette.gray),
    EntityIconColorOption("레드", "red", palette.red),
    EntityIconColorOption("오렌지", "orange", palette.orange),
    EntityIconColorOption("옐로", "yellow", palette.yellow),
    EntityIconColorOption("그린", "green", palette.green),
    EntityIconColorOption("블루", "blue", palette.blue),
    EntityIconColorOption("퍼플", "purple", palette.purple),
  )
}

fun resolveEntityIconAppearance(
  iconName: String?,
  iconColor: String?,
  fallbackIcon: IconData,
  fallbackTint: Color,
  colors: AppColors,
): EntityIconAppearance {
  return EntityIconAppearance(
    icon = entityIconMap[iconName?.trim()] ?: fallbackIcon,
    tint = resolveEntityIconTint(iconColor, colors) ?: fallbackTint,
  )
}

fun resolveEntityIconTint(iconColor: String?, colors: AppColors): Color? {
  return when (iconColor?.trim()) {
    "gray" -> colors.palette.gray
    "red" -> colors.palette.red
    "orange" -> colors.palette.orange
    "yellow" -> colors.palette.yellow
    "green" -> colors.palette.green
    "blue" -> colors.palette.blue
    "purple" -> colors.palette.purple
    else -> null
  }
}
