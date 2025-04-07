use std::fmt::Display;

use crate::imports::*;

/// The kind of icon to display. This is taken directly from the Feather icon
/// set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconType {
	/// <https://feathericons.com/?query=activity>
	Activity,
	/// <https://feathericons.com/?query=airplay>
	Airplay,
	/// <https://feathericons.com/?query=alert-circle>
	AlertCircle,
	/// <https://feathericons.com/?query=alert-octagon>
	AlertOctagon,
	/// <https://feathericons.com/?query=alert-triangle>
	AlertTriangle,
	/// <https://feathericons.com/?query=align-center>
	AlignCenter,
	/// <https://feathericons.com/?query=align-justify>
	AlignJustify,
	/// <https://feathericons.com/?query=align-left>
	AlignLeft,
	/// <https://feathericons.com/?query=align-right>
	AlignRight,
	/// <https://feathericons.com/?query=anchor>
	Anchor,
	/// <https://feathericons.com/?query=aperture>
	Aperture,
	/// <https://feathericons.com/?query=archive>
	Archive,
	/// <https://feathericons.com/?query=arrow-down-circle>
	ArrowDownCircle,
	/// <https://feathericons.com/?query=arrow-down-left>
	ArrowDownLeft,
	/// <https://feathericons.com/?query=arrow-down-right>
	ArrowDownRight,
	/// <https://feathericons.com/?query=arrow-down>
	ArrowDown,
	/// <https://feathericons.com/?query=arrow-left-circle>
	ArrowLeftCircle,
	/// <https://feathericons.com/?query=arrow-left>
	ArrowLeft,
	/// <https://feathericons.com/?query=arrow-right-circle>
	ArrowRightCircle,
	/// <https://feathericons.com/?query=arrow-right>
	ArrowRight,
	/// <https://feathericons.com/?query=arrow-up-circle>
	ArrowUpCircle,
	/// <https://feathericons.com/?query=arrow-up-left>
	ArrowUpLeft,
	/// <https://feathericons.com/?query=arrow-up-right>
	ArrowUpRight,
	/// <https://feathericons.com/?query=arrow-up>
	ArrowUp,
	/// <https://feathericons.com/?query=at-sign>
	AtSign,
	/// <https://feathericons.com/?query=award>
	Award,
	/// <https://feathericons.com/?query=bar-chart2>
	BarChart2,
	/// <https://feathericons.com/?query=bar-chart>
	BarChart,
	/// <https://feathericons.com/?query=battery-charging>
	BatteryCharging,
	/// <https://feathericons.com/?query=battery>
	Battery,
	/// <https://feathericons.com/?query=bell-off>
	BellOff,
	/// <https://feathericons.com/?query=bell>
	Bell,
	/// <https://feathericons.com/?query=bluetooth>
	Bluetooth,
	/// <https://feathericons.com/?query=bold>
	Bold,
	/// <https://feathericons.com/?query=book-open>
	BookOpen,
	/// <https://feathericons.com/?query=book>
	Book,
	/// <https://feathericons.com/?query=bookmark>
	Bookmark,
	/// <https://feathericons.com/?query=box>
	Box,
	/// <https://feathericons.com/?query=briefcase>
	Briefcase,
	/// <https://feathericons.com/?query=calendar>
	Calendar,
	/// <https://feathericons.com/?query=camera-off>
	CameraOff,
	/// <https://feathericons.com/?query=camera>
	Camera,
	/// <https://feathericons.com/?query=cast>
	Cast,
	/// <https://feathericons.com/?query=check-circle>
	CheckCircle,
	/// <https://feathericons.com/?query=check-square>
	CheckSquare,
	/// <https://feathericons.com/?query=check>
	Check,
	/// <https://feathericons.com/?query=chevron-down>
	ChevronDown,
	/// <https://feathericons.com/?query=chevron-left>
	ChevronLeft,
	/// <https://feathericons.com/?query=chevron-right>
	ChevronRight,
	/// <https://feathericons.com/?query=chevron-up>
	ChevronUp,
	/// <https://feathericons.com/?query=chevrons-down>
	ChevronsDown,
	/// <https://feathericons.com/?query=chevrons-left>
	ChevronsLeft,
	/// <https://feathericons.com/?query=chevrons-right>
	ChevronsRight,
	/// <https://feathericons.com/?query=chevrons-up>
	ChevronsUp,
	/// <https://feathericons.com/?query=chrome>
	Chrome,
	/// <https://feathericons.com/?query=circle>
	Circle,
	/// <https://feathericons.com/?query=clipboard>
	Clipboard,
	/// <https://feathericons.com/?query=clock>
	Clock,
	/// <https://feathericons.com/?query=cloud-drizzle>
	CloudDrizzle,
	/// <https://feathericons.com/?query=cloud-lightning>
	CloudLightning,
	/// <https://feathericons.com/?query=cloud-off>
	CloudOff,
	/// <https://feathericons.com/?query=cloud-rain>
	CloudRain,
	/// <https://feathericons.com/?query=cloud-snow>
	CloudSnow,
	/// <https://feathericons.com/?query=cloud>
	Cloud,
	/// <https://feathericons.com/?query=code>
	Code,
	/// <https://feathericons.com/?query=codepen>
	Codepen,
	/// <https://feathericons.com/?query=codesandbox>
	Codesandbox,
	/// <https://feathericons.com/?query=coffee>
	Coffee,
	/// <https://feathericons.com/?query=columns>
	Columns,
	/// <https://feathericons.com/?query=command>
	Command,
	/// <https://feathericons.com/?query=compass>
	Compass,
	/// <https://feathericons.com/?query=copy>
	Copy,
	/// <https://feathericons.com/?query=corner-down-left>
	CornerDownLeft,
	/// <https://feathericons.com/?query=corner-down-right>
	CornerDownRight,
	/// <https://feathericons.com/?query=corner-left-down>
	CornerLeftDown,
	/// <https://feathericons.com/?query=corner-left-up>
	CornerLeftUp,
	/// <https://feathericons.com/?query=corner-right-down>
	CornerRightDown,
	/// <https://feathericons.com/?query=corner-right-up>
	CornerRightUp,
	/// <https://feathericons.com/?query=corner-up-left>
	CornerUpLeft,
	/// <https://feathericons.com/?query=corner-up-right>
	CornerUpRight,
	/// <https://feathericons.com/?query=cpu>
	Cpu,
	/// <https://feathericons.com/?query=credit-card>
	CreditCard,
	/// <https://feathericons.com/?query=crop>
	Crop,
	/// <https://feathericons.com/?query=crosshair>
	Crosshair,
	/// <https://feathericons.com/?query=database>
	Database,
	/// <https://feathericons.com/?query=delete>
	Delete,
	/// <https://feathericons.com/?query=disc>
	Disc,
	/// <https://feathericons.com/?query=divide-circle>
	DivideCircle,
	/// <https://feathericons.com/?query=divide-square>
	DivideSquare,
	/// <https://feathericons.com/?query=divide>
	Divide,
	/// <https://feathericons.com/?query=dollar-sign>
	DollarSign,
	/// <https://feathericons.com/?query=download-cloud>
	DownloadCloud,
	/// <https://feathericons.com/?query=download>
	Download,
	/// <https://feathericons.com/?query=dribbble>
	Dribbble,
	/// <https://feathericons.com/?query=droplet>
	Droplet,
	/// <https://feathericons.com/?query=edit2>
	Edit2,
	/// <https://feathericons.com/?query=edit3>
	Edit3,
	/// <https://feathericons.com/?query=edit>
	Edit,
	/// <https://feathericons.com/?query=external-link>
	ExternalLink,
	/// <https://feathericons.com/?query=eye-off>
	EyeOff,
	/// <https://feathericons.com/?query=eye>
	Eye,
	/// <https://feathericons.com/?query=facebook>
	Facebook,
	/// <https://feathericons.com/?query=fast-forward>
	FastForward,
	/// <https://feathericons.com/?query=feather>
	Feather,
	/// <https://feathericons.com/?query=figma>
	Figma,
	/// <https://feathericons.com/?query=file-minus>
	FileMinus,
	/// <https://feathericons.com/?query=file-plus>
	FilePlus,
	/// <https://feathericons.com/?query=file-text>
	FileText,
	/// <https://feathericons.com/?query=file>
	File,
	/// <https://feathericons.com/?query=film>
	Film,
	/// <https://feathericons.com/?query=filter>
	Filter,
	/// <https://feathericons.com/?query=flag>
	Flag,
	/// <https://feathericons.com/?query=folder-minus>
	FolderMinus,
	/// <https://feathericons.com/?query=folder-plus>
	FolderPlus,
	/// <https://feathericons.com/?query=folder>
	Folder,
	/// <https://feathericons.com/?query=framer>
	Framer,
	/// <https://feathericons.com/?query=frown>
	Frown,
	/// <https://feathericons.com/?query=gift>
	Gift,
	/// <https://feathericons.com/?query=git-branch>
	GitBranch,
	/// <https://feathericons.com/?query=git-commit>
	GitCommit,
	/// <https://feathericons.com/?query=git-merge>
	GitMerge,
	/// <https://feathericons.com/?query=git-pull-request>
	GitPullRequest,
	/// <https://feathericons.com/?query=github>
	Github,
	/// <https://feathericons.com/?query=gitlab>
	Gitlab,
	/// <https://feathericons.com/?query=globe>
	Globe,
	/// <https://feathericons.com/?query=grid>
	Grid,
	/// <https://feathericons.com/?query=hard-drive>
	HardDrive,
	/// <https://feathericons.com/?query=hash>
	Hash,
	/// <https://feathericons.com/?query=headphones>
	Headphones,
	/// <https://feathericons.com/?query=heart>
	Heart,
	/// <https://feathericons.com/?query=help-circle>
	HelpCircle,
	/// <https://feathericons.com/?query=hexagon>
	Hexagon,
	/// <https://feathericons.com/?query=home>
	Home,
	/// <https://feathericons.com/?query=image>
	Image,
	/// <https://feathericons.com/?query=inbox>
	Inbox,
	/// <https://feathericons.com/?query=info>
	Info,
	/// <https://feathericons.com/?query=instagram>
	Instagram,
	/// <https://feathericons.com/?query=italic>
	Italic,
	/// <https://feathericons.com/?query=key>
	Key,
	/// <https://feathericons.com/?query=layers>
	Layers,
	/// <https://feathericons.com/?query=layout>
	Layout,
	/// <https://feathericons.com/?query=life-buoy>
	LifeBuoy,
	/// <https://feathericons.com/?query=link2>
	Link2,
	/// <https://feathericons.com/?query=link>
	Link,
	/// <https://feathericons.com/?query=linkedin>
	Linkedin,
	/// <https://feathericons.com/?query=list>
	List,
	/// <https://feathericons.com/?query=loader>
	Loader,
	/// <https://feathericons.com/?query=lock>
	Lock,
	/// <https://feathericons.com/?query=log-in>
	LogIn,
	/// <https://feathericons.com/?query=log-out>
	LogOut,
	/// <https://feathericons.com/?query=mail>
	Mail,
	/// <https://feathericons.com/?query=map-pin>
	MapPin,
	/// <https://feathericons.com/?query=map>
	Map,
	/// <https://feathericons.com/?query=maximize2>
	Maximize2,
	/// <https://feathericons.com/?query=maximize>
	Maximize,
	/// <https://feathericons.com/?query=meh>
	Meh,
	/// <https://feathericons.com/?query=menu>
	Menu,
	/// <https://feathericons.com/?query=message-circle>
	MessageCircle,
	/// <https://feathericons.com/?query=message-square>
	MessageSquare,
	/// <https://feathericons.com/?query=mic-off>
	MicOff,
	/// <https://feathericons.com/?query=mic>
	Mic,
	/// <https://feathericons.com/?query=minimize2>
	Minimize2,
	/// <https://feathericons.com/?query=minimize>
	Minimize,
	/// <https://feathericons.com/?query=minus-circle>
	MinusCircle,
	/// <https://feathericons.com/?query=minus-square>
	MinusSquare,
	/// <https://feathericons.com/?query=minus>
	Minus,
	/// <https://feathericons.com/?query=monitor>
	Monitor,
	/// <https://feathericons.com/?query=moon>
	Moon,
	/// <https://feathericons.com/?query=more-horizontal>
	MoreHorizontal,
	/// <https://feathericons.com/?query=more-vertical>
	MoreVertical,
	/// <https://feathericons.com/?query=mouse-pointer>
	MousePointer,
	/// <https://feathericons.com/?query=move>
	Move,
	/// <https://feathericons.com/?query=music>
	Music,
	/// <https://feathericons.com/?query=navigation2>
	Navigation2,
	/// <https://feathericons.com/?query=navigation>
	Navigation,
	/// <https://feathericons.com/?query=octagon>
	Octagon,
	/// <https://feathericons.com/?query=package>
	Package,
	/// <https://feathericons.com/?query=paperclip>
	Paperclip,
	/// <https://feathericons.com/?query=pause-circle>
	PauseCircle,
	/// <https://feathericons.com/?query=pause>
	Pause,
	/// <https://feathericons.com/?query=pen-tool>
	PenTool,
	/// <https://feathericons.com/?query=percent>
	Percent,
	/// <https://feathericons.com/?query=phone-call>
	PhoneCall,
	/// <https://feathericons.com/?query=phone-forwarded>
	PhoneForwarded,
	/// <https://feathericons.com/?query=phone-incoming>
	PhoneIncoming,
	/// <https://feathericons.com/?query=phone-missed>
	PhoneMissed,
	/// <https://feathericons.com/?query=phone-off>
	PhoneOff,
	/// <https://feathericons.com/?query=phone-outgoing>
	PhoneOutgoing,
	/// <https://feathericons.com/?query=phone>
	Phone,
	/// <https://feathericons.com/?query=pie-chart>
	PieChart,
	/// <https://feathericons.com/?query=play-circle>
	PlayCircle,
	/// <https://feathericons.com/?query=play>
	Play,
	/// <https://feathericons.com/?query=plus-circle>
	PlusCircle,
	/// <https://feathericons.com/?query=plus-square>
	PlusSquare,
	/// <https://feathericons.com/?query=plus>
	Plus,
	/// <https://feathericons.com/?query=pocket>
	Pocket,
	/// <https://feathericons.com/?query=power>
	Power,
	/// <https://feathericons.com/?query=printer>
	Printer,
	/// <https://feathericons.com/?query=radio>
	Radio,
	/// <https://feathericons.com/?query=refresh-ccw>
	RefreshCcw,
	/// <https://feathericons.com/?query=refresh-cw>
	RefreshCw,
	/// <https://feathericons.com/?query=repeat>
	Repeat,
	/// <https://feathericons.com/?query=rewind>
	Rewind,
	/// <https://feathericons.com/?query=rotate-ccw>
	RotateCcw,
	/// <https://feathericons.com/?query=rotate-cw>
	RotateCw,
	/// <https://feathericons.com/?query=rss>
	Rss,
	/// <https://feathericons.com/?query=save>
	Save,
	/// <https://feathericons.com/?query=scissors>
	Scissors,
	/// <https://feathericons.com/?query=search>
	Search,
	/// <https://feathericons.com/?query=send>
	Send,
	/// <https://feathericons.com/?query=server>
	Server,
	/// <https://feathericons.com/?query=settings>
	Settings,
	/// <https://feathericons.com/?query=share2>
	Share2,
	/// <https://feathericons.com/?query=share>
	Share,
	/// <https://feathericons.com/?query=shield-off>
	ShieldOff,
	/// <https://feathericons.com/?query=shield>
	Shield,
	/// <https://feathericons.com/?query=shopping-bag>
	ShoppingBag,
	/// <https://feathericons.com/?query=shopping-cart>
	ShoppingCart,
	/// <https://feathericons.com/?query=shuffle>
	Shuffle,
	/// <https://feathericons.com/?query=sidebar>
	Sidebar,
	/// <https://feathericons.com/?query=skip-back>
	SkipBack,
	/// <https://feathericons.com/?query=skip-forward>
	SkipForward,
	/// <https://feathericons.com/?query=slack>
	Slack,
	/// <https://feathericons.com/?query=slash>
	Slash,
	/// <https://feathericons.com/?query=sliders>
	Sliders,
	/// <https://feathericons.com/?query=smartphone>
	Smartphone,
	/// <https://feathericons.com/?query=smile>
	Smile,
	/// <https://feathericons.com/?query=speaker>
	Speaker,
	/// <https://feathericons.com/?query=square>
	Square,
	/// <https://feathericons.com/?query=star>
	Star,
	/// <https://feathericons.com/?query=stop-circle>
	StopCircle,
	/// <https://feathericons.com/?query=sun>
	Sun,
	/// <https://feathericons.com/?query=sunrise>
	Sunrise,
	/// <https://feathericons.com/?query=sunset>
	Sunset,
	/// <https://feathericons.com/?query=table>
	Table,
	/// <https://feathericons.com/?query=tablet>
	Tablet,
	/// <https://feathericons.com/?query=tag>
	Tag,
	/// <https://feathericons.com/?query=target>
	Target,
	/// <https://feathericons.com/?query=terminal>
	Terminal,
	/// <https://feathericons.com/?query=thermometer>
	Thermometer,
	/// <https://feathericons.com/?query=thumbs-down>
	ThumbsDown,
	/// <https://feathericons.com/?query=thumbs-up>
	ThumbsUp,
	/// <https://feathericons.com/?query=toggle-left>
	ToggleLeft,
	/// <https://feathericons.com/?query=toggle-right>
	ToggleRight,
	/// <https://feathericons.com/?query=tool>
	Tool,
	/// <https://feathericons.com/?query=trash2>
	Trash2,
	/// <https://feathericons.com/?query=trash>
	Trash,
	/// <https://feathericons.com/?query=trello>
	Trello,
	/// <https://feathericons.com/?query=trending-down>
	TrendingDown,
	/// <https://feathericons.com/?query=trending-up>
	TrendingUp,
	/// <https://feathericons.com/?query=triangle>
	Triangle,
	/// <https://feathericons.com/?query=truck>
	Truck,
	/// <https://feathericons.com/?query=tv>
	Tv,
	/// <https://feathericons.com/?query=twitch>
	Twitch,
	/// <https://feathericons.com/?query=twitter>
	Twitter,
	/// <https://feathericons.com/?query=type>
	Type,
	/// <https://feathericons.com/?query=umbrella>
	Umbrella,
	/// <https://feathericons.com/?query=underline>
	Underline,
	/// <https://feathericons.com/?query=unlock>
	Unlock,
	/// <https://feathericons.com/?query=upload-cloud>
	UploadCloud,
	/// <https://feathericons.com/?query=upload>
	Upload,
	/// <https://feathericons.com/?query=user-check>
	UserCheck,
	/// <https://feathericons.com/?query=user-minus>
	UserMinus,
	/// <https://feathericons.com/?query=user-plus>
	UserPlus,
	/// <https://feathericons.com/?query=user-x>
	UserX,
	/// <https://feathericons.com/?query=user>
	User,
	/// <https://feathericons.com/?query=users>
	Users,
	/// <https://feathericons.com/?query=video-off>
	VideoOff,
	/// <https://feathericons.com/?query=video>
	Video,
	/// <https://feathericons.com/?query=voicemail>
	Voicemail,
	/// <https://feathericons.com/?query=volume1>
	Volume1,
	/// <https://feathericons.com/?query=volume2>
	Volume2,
	/// <https://feathericons.com/?query=volume-x>
	VolumeX,
	/// <https://feathericons.com/?query=volume>
	Volume,
	/// <https://feathericons.com/?query=watch>
	Watch,
	/// <https://feathericons.com/?query=wifi-off>
	WifiOff,
	/// <https://feathericons.com/?query=wifi>
	Wifi,
	/// <https://feathericons.com/?query=wind>
	Wind,
	/// <https://feathericons.com/?query=x-circle>
	XCircle,
	/// <https://feathericons.com/?query=x-octagon>
	XOctagon,
	/// <https://feathericons.com/?query=x-square>
	XSquare,
	/// <https://feathericons.com/?query=x>
	X,
	/// <https://feathericons.com/?query=youtube>
	Youtube,
	/// <https://feathericons.com/?query=zap-off>
	ZapOff,
	/// <https://feathericons.com/?query=zap>
	Zap,
	/// <https://feathericons.com/?query=zoom-in>
	ZoomIn,
	/// <https://feathericons.com/?query=zoom-out>
	ZoomOut,
}

impl Display for IconType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Activity => "activity",
				Self::Airplay => "airplay",
				Self::AlertCircle => "alert-circle",
				Self::AlertOctagon => "alert-octagon",
				Self::AlertTriangle => "alert-triangle",
				Self::AlignCenter => "align-center",
				Self::AlignJustify => "align-justify",
				Self::AlignLeft => "align-left",
				Self::AlignRight => "align-right",
				Self::Anchor => "anchor",
				Self::Aperture => "aperture",
				Self::Archive => "archive",
				Self::ArrowDownCircle => "arrow-down-circle",
				Self::ArrowDownLeft => "arrow-down-left",
				Self::ArrowDownRight => "arrow-down-right",
				Self::ArrowDown => "arrow-down",
				Self::ArrowLeftCircle => "arrow-left-circle",
				Self::ArrowLeft => "arrow-left",
				Self::ArrowRightCircle => "arrow-right-circle",
				Self::ArrowRight => "arrow-right",
				Self::ArrowUpCircle => "arrow-up-circle",
				Self::ArrowUpLeft => "arrow-up-left",
				Self::ArrowUpRight => "arrow-up-right",
				Self::ArrowUp => "arrow-up",
				Self::AtSign => "at-sign",
				Self::Award => "award",
				Self::BarChart2 => "bar-chart-2",
				Self::BarChart => "bar-chart",
				Self::BatteryCharging => "battery-charging",
				Self::Battery => "battery",
				Self::BellOff => "bell-off",
				Self::Bell => "bell",
				Self::Bluetooth => "bluetooth",
				Self::Bold => "bold",
				Self::BookOpen => "book-open",
				Self::Book => "book",
				Self::Bookmark => "bookmark",
				Self::Box => "box",
				Self::Briefcase => "briefcase",
				Self::Calendar => "calendar",
				Self::CameraOff => "camera-off",
				Self::Camera => "camera",
				Self::Cast => "cast",
				Self::CheckCircle => "check-circle",
				Self::CheckSquare => "check-square",
				Self::Check => "check",
				Self::ChevronDown => "chevron-down",
				Self::ChevronLeft => "chevron-left",
				Self::ChevronRight => "chevron-right",
				Self::ChevronUp => "chevron-up",
				Self::ChevronsDown => "chevrons-down",
				Self::ChevronsLeft => "chevrons-left",
				Self::ChevronsRight => "chevrons-right",
				Self::ChevronsUp => "chevrons-up",
				Self::Chrome => "chrome",
				Self::Circle => "circle",
				Self::Clipboard => "clipboard",
				Self::Clock => "clock",
				Self::CloudDrizzle => "cloud-drizzle",
				Self::CloudLightning => "cloud-lightning",
				Self::CloudOff => "cloud-off",
				Self::CloudRain => "cloud-rain",
				Self::CloudSnow => "cloud-snow",
				Self::Cloud => "cloud",
				Self::Code => "code",
				Self::Codepen => "codepen",
				Self::Codesandbox => "codesandbox",
				Self::Coffee => "coffee",
				Self::Columns => "columns",
				Self::Command => "command",
				Self::Compass => "compass",
				Self::Copy => "copy",
				Self::CornerDownLeft => "corner-down-left",
				Self::CornerDownRight => "corner-down-right",
				Self::CornerLeftDown => "corner-left-down",
				Self::CornerLeftUp => "corner-left-up",
				Self::CornerRightDown => "corner-right-down",
				Self::CornerRightUp => "corner-right-up",
				Self::CornerUpLeft => "corner-up-left",
				Self::CornerUpRight => "corner-up-right",
				Self::Cpu => "cpu",
				Self::CreditCard => "credit-card",
				Self::Crop => "crop",
				Self::Crosshair => "crosshair",
				Self::Database => "database",
				Self::Delete => "delete",
				Self::Disc => "disc",
				Self::DivideCircle => "divide-circle",
				Self::DivideSquare => "divide-square",
				Self::Divide => "divide",
				Self::DollarSign => "dollar-sign",
				Self::DownloadCloud => "download-cloud",
				Self::Download => "download",
				Self::Dribbble => "dribbble",
				Self::Droplet => "droplet",
				Self::Edit2 => "edit-2",
				Self::Edit3 => "edit-3",
				Self::Edit => "edit",
				Self::ExternalLink => "external-link",
				Self::EyeOff => "eye-off",
				Self::Eye => "eye",
				Self::Facebook => "facebook",
				Self::FastForward => "fast-forward",
				Self::Feather => "feather",
				Self::Figma => "figma",
				Self::FileMinus => "file-minus",
				Self::FilePlus => "file-plus",
				Self::FileText => "file-text",
				Self::File => "file",
				Self::Film => "film",
				Self::Filter => "filter",
				Self::Flag => "flag",
				Self::FolderMinus => "folder-minus",
				Self::FolderPlus => "folder-plus",
				Self::Folder => "folder",
				Self::Framer => "framer",
				Self::Frown => "frown",
				Self::Gift => "gift",
				Self::GitBranch => "git-branch",
				Self::GitCommit => "git-commit",
				Self::GitMerge => "git-merge",
				Self::GitPullRequest => "git-pull-request",
				Self::Github => "github",
				Self::Gitlab => "gitlab",
				Self::Globe => "globe",
				Self::Grid => "grid",
				Self::HardDrive => "hard-drive",
				Self::Hash => "hash",
				Self::Headphones => "headphones",
				Self::Heart => "heart",
				Self::HelpCircle => "help-circle",
				Self::Hexagon => "hexagon",
				Self::Home => "home",
				Self::Image => "image",
				Self::Inbox => "inbox",
				Self::Info => "info",
				Self::Instagram => "instagram",
				Self::Italic => "italic",
				Self::Key => "key",
				Self::Layers => "layers",
				Self::Layout => "layout",
				Self::LifeBuoy => "life-buoy",
				Self::Link2 => "link-2",
				Self::Link => "link",
				Self::Linkedin => "linkedin",
				Self::List => "list",
				Self::Loader => "loader",
				Self::Lock => "lock",
				Self::LogIn => "log-in",
				Self::LogOut => "log-out",
				Self::Mail => "mail",
				Self::MapPin => "map-pin",
				Self::Map => "map",
				Self::Maximize2 => "maximize-2",
				Self::Maximize => "maximize",
				Self::Meh => "meh",
				Self::Menu => "menu",
				Self::MessageCircle => "message-circle",
				Self::MessageSquare => "message-square",
				Self::MicOff => "mic-off",
				Self::Mic => "mic",
				Self::Minimize2 => "minimize-2",
				Self::Minimize => "minimize",
				Self::MinusCircle => "minus-circle",
				Self::MinusSquare => "minus-square",
				Self::Minus => "minus",
				Self::Monitor => "monitor",
				Self::Moon => "moon",
				Self::MoreHorizontal => "more-horizontal",
				Self::MoreVertical => "more-vertical",
				Self::MousePointer => "mouse-pointer",
				Self::Move => "move",
				Self::Music => "music",
				Self::Navigation2 => "navigation-2",
				Self::Navigation => "navigation",
				Self::Octagon => "octagon",
				Self::Package => "package",
				Self::Paperclip => "paperclip",
				Self::PauseCircle => "pause-circle",
				Self::Pause => "pause",
				Self::PenTool => "pen-tool",
				Self::Percent => "percent",
				Self::PhoneCall => "phone-call",
				Self::PhoneForwarded => "phone-forwarded",
				Self::PhoneIncoming => "phone-incoming",
				Self::PhoneMissed => "phone-missed",
				Self::PhoneOff => "phone-off",
				Self::PhoneOutgoing => "phone-outgoing",
				Self::Phone => "phone",
				Self::PieChart => "pie-chart",
				Self::PlayCircle => "play-circle",
				Self::Play => "play",
				Self::PlusCircle => "plus-circle",
				Self::PlusSquare => "plus-square",
				Self::Plus => "plus",
				Self::Pocket => "pocket",
				Self::Power => "power",
				Self::Printer => "printer",
				Self::Radio => "radio",
				Self::RefreshCcw => "refresh-ccw",
				Self::RefreshCw => "refresh-cw",
				Self::Repeat => "repeat",
				Self::Rewind => "rewind",
				Self::RotateCcw => "rotate-ccw",
				Self::RotateCw => "rotate-cw",
				Self::Rss => "rss",
				Self::Save => "save",
				Self::Scissors => "scissors",
				Self::Search => "search",
				Self::Send => "send",
				Self::Server => "server",
				Self::Settings => "settings",
				Self::Share2 => "share-2",
				Self::Share => "share",
				Self::ShieldOff => "shield-off",
				Self::Shield => "shield",
				Self::ShoppingBag => "shopping-bag",
				Self::ShoppingCart => "shopping-cart",
				Self::Shuffle => "shuffle",
				Self::Sidebar => "sidebar",
				Self::SkipBack => "skip-back",
				Self::SkipForward => "skip-forward",
				Self::Slack => "slack",
				Self::Slash => "slash",
				Self::Sliders => "sliders",
				Self::Smartphone => "smartphone",
				Self::Smile => "smile",
				Self::Speaker => "speaker",
				Self::Square => "square",
				Self::Star => "star",
				Self::StopCircle => "stop-circle",
				Self::Sun => "sun",
				Self::Sunrise => "sunrise",
				Self::Sunset => "sunset",
				Self::Table => "table",
				Self::Tablet => "tablet",
				Self::Tag => "tag",
				Self::Target => "target",
				Self::Terminal => "terminal",
				Self::Thermometer => "thermometer",
				Self::ThumbsDown => "thumbs-down",
				Self::ThumbsUp => "thumbs-up",
				Self::ToggleLeft => "toggle-left",
				Self::ToggleRight => "toggle-right",
				Self::Tool => "tool",
				Self::Trash2 => "trash-2",
				Self::Trash => "trash",
				Self::Trello => "trello",
				Self::TrendingDown => "trending-down",
				Self::TrendingUp => "trending-up",
				Self::Triangle => "triangle",
				Self::Truck => "truck",
				Self::Tv => "tv",
				Self::Twitch => "twitch",
				Self::Twitter => "twitter",
				Self::Type => "type",
				Self::Umbrella => "umbrella",
				Self::Underline => "underline",
				Self::Unlock => "unlock",
				Self::UploadCloud => "upload-cloud",
				Self::Upload => "upload",
				Self::UserCheck => "user-check",
				Self::UserMinus => "user-minus",
				Self::UserPlus => "user-plus",
				Self::UserX => "user-x",
				Self::User => "user",
				Self::Users => "users",
				Self::VideoOff => "video-off",
				Self::Video => "video",
				Self::Voicemail => "voicemail",
				Self::Volume1 => "volume-1",
				Self::Volume2 => "volume-2",
				Self::VolumeX => "volume-x",
				Self::Volume => "volume",
				Self::Watch => "watch",
				Self::WifiOff => "wifi-off",
				Self::Wifi => "wifi",
				Self::Wind => "wind",
				Self::XCircle => "x-circle",
				Self::XOctagon => "x-octagon",
				Self::XSquare => "x-square",
				Self::X => "x",
				Self::Youtube => "youtube",
				Self::ZapOff => "zap-off",
				Self::Zap => "zap",
				Self::ZoomIn => "zoom-in",
				Self::ZoomOut => "zoom-out",
			}
		)
	}
}

#[component]
pub fn icon(
	/// name of the icon to display
	#[prop(into)]
	icon: MaybeSignal<IconType>,
	/// class name to apply to the icon
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// text color of the icon
	#[prop(into, optional, default = Color::White.into())]
	color: MaybeSignal<Color>,
	/// fill color of the icon
	#[prop(into, optional)]
	fill: MaybeSignal<Color>,
	/// size of the icon
	#[prop(into, optional)]
	size: MaybeSignal<Size>,
	/// Whether to enable the pulse animation
	#[prop(into, optional, default = false.into())]
	enable_pulse: MaybeSignal<bool>,
	/// click handler
	#[prop(optional)]
	on_click: Option<ClickHandler>,
) -> impl IntoView {
	let is_clickable = on_click.is_some();

	view! {
		<svg
			class={move || {
				format!(
					"icon {} {} icon-fill-{} icon-{} {} {}",
					if enable_pulse.get() { "pulse" } else { "" },
					color.get().as_text_color().as_css_color(),
					fill.get().as_css_name(),
					size.get().as_css_name(),
					if is_clickable { "cursor-pointer" } else { "" },
					class.get(),
				)
			}}

			on:click={move |e| {
				if let Some(click) = on_click.clone() {
					click(&e)
				}
			}}
		>

			<use_ href={move || format!("{}#{}", constants::FEATHER_IMG, icon.get())}></use_>
		</svg>
	}
}
