# Changelog

What changed in each release of WestPlay. The newest release's notes are what the app shows when it offers the
update.

## 3.0.1 (2026-11-25)

### 新功能

- **沃尔沃 9700 DD 屏幕适配** —— 感谢 **@BoboCheese74**（[PR #9](https://github.com/LucianLuu/WestPlay-CN-Vision/pull/9)）。
  客运 DLC 的这辆客车，中控屏由 WestPlay 接管：它自己的屏幕脚本
  （`volvo_9700_23_gps.sii`），出厂 1024 x 512 的表面，画面在整个表面里每边留 0.32%。

### 修复

- **网易云 / QQ 音乐加载不出来** —— 一位玩家写了个 `.bat`：杀掉中间层再重启就好。
  这说中了根因 —— 两个中间层是独立进程，会停（被清理工具杀掉、被 Windows 回收内存、
  或者自己出错），而应用从来不知道，只顾着问一个没人听的端口。
  现在应用每 20 秒问它们一次：进程还在、健康接口也应答，才算是活的；不在了就自己重新
  启动。实测手动 kill 之后，网易云 5 秒、QQ 音乐 8 秒回来。
  失败 5 次就停手，免得安装残缺时一直去启动一个注定失败的进程。

- **哔哩哔哩接入空间音频** —— 声音从卡车的扬声器出来，转头不再断（之前整张音频图会被
  重接一次，切断音源）。上游 3.0 给 B 站留的五条命令（扬声器、头部、车外、音调、频谱）
  都写好了，只是从来没连上，接的时候发现断在五个地方，都修了。

- **哔哩哔哩的仪表盘卡片** —— 视频的封面和标题放进「正在播放」，和音乐软件同一张卡片，
  不再是铺满的视频画面（YouTube 的仍是画面）。

- **哔哩哔哩的音量/暂停/切集键** —— 它现在是应用认得的播放器之一：音量键、暂停键、
  长按切集都作用于它，切到它时别的播放器会暂停。

## 3.0.0 (2026-11-24)

### 新功能

- **界面焕新** —— 软件界面换成全新的液态玻璃 UI，更简洁、更流畅：设置向导、账号、地图各页
  重新排布，玻璃面板跟着壁纸取色。
- **哔哩哔哩座舱客户端** —— 感谢 **[@jntm-5686](https://github.com/jntm-5686)**（Westplay-improved-version）。
  不再是「网页套壳」：首页按它自己的左栏 + 标签分栏，播放页换成它的深色栏与底部控件，
  右侧带简介 / 评论，选集能真的换分P，清晰度可选，另外补上直播、我的、消息、动态。
- **中文输入法候选条** —— 感谢 **[@jntm-5686](https://github.com/jntm-5686)**（Westplay-improved-version）。
  候选条按手机输入法的样子重做：左边是正在拼的拼音（按音节分开），右边候选字带数字角标，
  两端翻页键；一页九个候选（数字键 1–9 直接选），字库 720 KB 随程序打包。
- **YTHQ 屏幕适配** —— 感谢 **[@jntm-5686](https://github.com/jntm-5686)**（Westplay-improved-version）。
  装好 YTHQ 平板后自动识别为 WestPlay 屏幕，不再需要手动挑：四组补丁按游戏与车型自动匹配
  （ETS2 / ATS，含福特与 YTHQ 自己的屏幕脚本），平板画面层内缩、倒车影像提层。

### 修复

- **高速行驶时声音断断续续** —— 判断「人在车里还是在车外」原来还看相机离司机的距离，
  而相机位置和卡车位置是隔一帧读的：130 km/h 时距离漂移半米，三秒内穿过阈值两百多次，
  声音就在两套滤波之间来回切。现在只按屏幕画不画来判断（和上游 2.3.0 / 2.4.1 / 3.0 一致）。
- **Edge WebView2 导致软件崩溃** —— 158 版在设置向导旁的车机预览里打开应用会让整页变白
  （`STATUS_BREAKPOINT`）。规避（结束有限动画、切换时关掉预览外框的倒影）上游 3.0 里就有，
  但它只存在于主窗口加载的那份文件里，而主窗口的地址没有 `?demo`，安装函数第一行就返回了 ——
  真正会崩的 iframe 从来没拿到这段代码。现在写进源码（`ui/src/cabin/demo.ts`），并加了
  `verify_158.py` 在构建后校验它在 iframe 会跑的那个 bundle 里。
- **崩溃之后卡在错误页** —— 新增心跳：预览页每秒给主程序报一次「还在」，六秒没有就认为它死了，
  自动重载界面。之前只数进程，而 158 杀的是渲染进程、浏览器进程还活着，所以从来察觉不到。
  顺带修了「关掉登录窗口就自动刷新」—— 进程列表分不清是谁的进程，现在只看心跳。
- **网易云音乐后台播放完不切歌** —— 窗口开合时清理把所有监听器都解了，`onended` 也在内，
  于是歌播完没有任何东西接下一首。现在只解「窗口画图用的」，播放相关的一律留着。
- **QQ 音乐和网易云音乐加载不出内容** —— 两个原因。一是部署少了两个中间层目录
  （`netease-layer/`、`qq-music-layer/`，含各自的 Python 运行时），现在随安装包一起装。
  二是 QQ 音乐的端口判断只测「能不能连上」，而某些程序（主板控制面板之类）会占着一个端口
  却不监听 —— 现在两个都测：连得上、或者绑不上，都算占用，自动换下一个。
- **拼音输入法不能用** —— 启用条件原来要求输入框带 `playerTypes: false`，而车机自己的输入框
  （Apple Music、网易云、QQ 音乐、电台）都没这个字段，于是输入法在那些框里永远不出现。
  另外候选字送不到网页里的输入框：原来走浏览器的 `Input.insertText`，它只落在浏览器自己
  聚焦的框里，而哔哩哔哩那些搜索框是页面自己的组件，浏览器从不认为它们被聚焦 —— 现在改走
  页内消息，由页面自己插字。

### 其他

- **账号页** —— YouTube / Spotify / Apple Music 三个换成哔哩哔哩 / 网易云音乐 / QQ 音乐，
  登录状态改成**实时查询**（各自问自己的客户端或中间层），不再重启就丢。
- **网易云音乐** —— 官方红底音符标志；全屏歌词和歌名改成白字（之前深色字沉在深色背景里）；
  QQ 音乐 VIP 歌曲能播了（换成带文件名的接口，并补上站点原名的 cookie）。
- **帧率** —— 3.0 界面的「帧率」和车的设置页里那个合并成一套。

### 致谢

这一版的 YTHQ 屏幕适配、中文输入法候选条、哔哩哔哩座舱客户端，主要代码都来自
**[@jntm-5686](https://github.com/jntm-5686)** 的
[Westplay-improved-version](https://github.com/jntm-5686/Westplay-improved-version)，
按上游 GPL-3.0-or-later 使用，原样或最小改动并入。谢谢！

## 2.4.1+2 (2026-11-20)

### 新功能

- **网易云音乐重写** —— 感谢 **@BoboCheese74**：不再是一个网页版（`music.163.com` 整页 WebView），
  而是座舱自己的应用 —— 自己的首页、歌单、搜索、歌词页与「正在播放」，和 Apple Music、QQ 音乐
  一致；扫码登录；音量、播放控制、与游戏的联动也都在内。中间是随程序一起打包的本地服务
  （只用 Python 标准库，约 18 MB），账号与请求都在本机完成。
- **仪表盘的「正在播放」** 认得网易云了：歌名、封面、三个按钮，和 QQ 音乐那张卡片一样。
- **Coaches 客运导航** —— 感谢 **@BoboCheese74**（PR #6）：往返任务会把**全部剩余途经站**标出来，
  只高亮到下一站，后续路线用较暗的颜色；停靠完成后自动移走本站、切到下一站。

### 修复

- **应用图标不见了** —— 上一版把图标打包成了浏览器读不了的格式（内嵌图片写成了不合法的
  `data:img/png`），全新安装的玩家首页一片空白只有文字。现在图标能正常显示，并且从 8.1 MB
  压到 0.56 MB（首页要同时画 9 个，原来每次打开都要解码 1.4 MB）。
- **QQ 音乐 VIP 歌曲放不了**（`104003`）—— 中间层要播放地址时没带文件名，站点因此判成没有权限。
  现在改用座舱自己的接口去要，带上文件名，VIP 歌曲正常播放。
- **QQ 音乐退出后重新登录还是自己** —— 退出时逐个删 cookie 会漏掉「记住我」，登录页一开就续上了。
  现在整个站点数据一起清（和 lingbz112 的 WestPlay-CN 一样），清完再开窗口。
- **QQ 音乐的空间音频没了** —— 上一版加的跨域探测在无源页面里对任何站点都答「拒绝」，
  所有歌都被推去普通播放。现在回到 2.4.1 的判定。
- **第三人称看车外声音不变闷** —— 原来靠「屏幕是否被绘制」判断，第三人称时屏幕常被画着。
  现在按相机到座舱的实际距离判断，第三人称、头部探出车窗都会变闷。
- **首页少了「今日为你打造」那一排**（每日30首、百万收藏、歌手漫游、新歌推荐）—— 去掉「每日30首」
  那张卡时把整个货架一起丢了。现在只去那一张，其余保留；首页顺序按 QQ 音乐网页版来。
- **网易云的四个播放松懈**：新建的音频上下文从不唤醒（无声）；点歌只交地址不播放（要手动暂停再播）；
  进度条停在 0:00（音频事件从未订阅）；暂停后切歌没声音（淡出没结束）。

## 2.4.1+1 (2026-11-18)

### 新功能

- **Live Radio（电台应用）** —— 感谢 **@juner122king**（PR #4）：一个独立的电台应用，在宿主的隐藏页里
  跑喜马拉雅电台目录，HLS 直播流，有自己的封面、节目表与「正在播放」页。它取代了原来的蜻蜓 FM：
  蜻蜓 FM 是网页版（`qtfm.cn` 一个整页 WebView），电台是宿主自己的实现，界面与座舱一致 ——
  （owner 2026-11-18 决定：把蜻蜓 FM 直接换成 Live FM）。
- **电台图标重绘** —— 感谢 **@GooseLLL**：Live Radio 的图标原来是一张手写 SVG（476 B，浅色与深色
  还是同一张图），和其余 Apple 风格图标的观感差得远。现在按图标集的规格重绘为 1024×1024、
  浅/深各一版，配色互为反色，与其它图标同一量级。
- **局域网手机遥控（LAN WebUI）** —— 感谢 **@LahantziBade**（PR #3）：可选开启，用手机浏览器
  在同一局域网内控制座舱（点按/滑动），设置里有配对地址与开关，默认关闭。

### 修复

- **「显示屏优先」开关重新生效**：设置 › 屏幕 里的平板优先开关（`tablet_first`）在同步上游 2.4.1 后
  拨了没反应 —— 上游重写 `crates/plugin/src/screens.rs` 时换了判定（谁先画出来谁上），我们原来
  「玩家指定谁优先」的逻辑被覆盖掉了，只剩设置项和桥接字段。现在两条规则合并：开关开（默认）平板优先，
  和 2.4.0 一致；开关关车机优先；大巴里保持上游的新行为（平板接管后大巴自己的屏幕还原成游戏的）。
- **QQ 音乐「载入失败」与 HTTP 429** —— 感谢 **@ManDik-Chan**（PR #5）：
  8080 被 Steam 的 `steamwebhelper` 占着时，原来会误以为中间层已在运行而不启动。现在首选 8787，
  并先确认应答的是不是我们的层（`/openapi.json` 的 `QQMusic API`），不是就顺延到下一个空闲端口，
  日志写明被谁占用。
  429 是中间层自己限流（每地址 60 次/60 秒）：一屏首页要发几十个请求，超了就载入失败。
  现在把回环地址放进 `rate_limit_exempt_ips`，本机请求不再被限流
  （实测：不带豁免 60 次 200 后全是 429；带豁免 120 次全 200）。
- QQ 音乐的空间音频：播放前先探测歌曲地址是否允许跨域，不允许就普通播放（浏览器报
  `MEDIA_ELEMENT_ERROR: Format error` 的那批玩家）；万一仍然失败，自动放弃空间音频、用普通方式
  重播这一首，不再整首歌放不出来。

## 2.4.1 (2026-11-17)
- Liquid Glass, as on iOS 26: the sidebar, the Dashboard's widgets, Apple Music's bar, buttons, mini player and
  menus, the map's buttons and search, the keyboard and the volume pill are glass again, made the way Apple's is.
  Clearer, so what's behind them shows through, softened; at their edge the glass bends what's behind it like a lens,
  and the edge's shine is the light of what's behind it (bright over a bright wallpaper, all but gone over a dark
  one), never a white line. Glass appears and goes by its frost and bend coming in, rather than fading; pressed, it
  lights up under the cursor and gives a little. The sidebar and the widgets stay readable over any wallpaper, in
  either look. In Apple Music, a page blurs softly as it scrolls under the bar and the mini player.
- Reduce Transparency (Settings › Appearance), as on the iPhone: the sidebar and every glass panel turn flat and
  solid, nothing behind them showing through or blurred.
- The music apps fit every screen: Now Playing's cover takes at most about a third of the screen in Apple Music,
  Spotify, YouTube Music and Player, so the song and its controls have room (on the MAN Lion's Coach's screen the
  artist was cut and repeat went off the screen). YouTube Music's Up next, Lyrics and Related open beside the
  controls instead of over them. On the Scania 7", Apple Music's, Spotify's and YouTube Music's bars, headers and
  tiles no longer run over each other.
- In a bus, the WestPlay tablet now shows WestPlay instead of the bus's own screen: fit the tablet in a coach of the
  Coaches DLC or in a bus mod with a WestPlay screen and, once the tablet comes into view, WestPlay moves onto it and
  the bus's screen shows the game's navigation and parking camera again, until you change vehicles. Without a tablet,
  WestPlay stays on the bus's screen. The bus mods with their own WestPlay screen now get the tablet too.
- Oyuncuyusbis' coaches now show WestPlay on their GPS part, as the Setra S 516 HD/2 already did: the Travego 16 SHD's
  tablet, the Setra S 531 DT's GPS and, in the MAN Lion's Coach, the game's portable navigator, each laid out in its
  own screen's shape. Their own dashboard screens show the game's navigation again, and WestPlay no longer copies
  anything of these bus mods.
- The WestPlay tablet now shows WestPlay in the Mercedes-Benz Travego 16 SHD, on TruckersMP too.
- A small game update (a hotfix, as 1.62.3.2 to 1.62.3.3) no longer prepares every map again: only a new release of
  the game (as 1.63) does. Maps already prepared for 1.62 stay as they are.
- A map with PL Rebuilding no longer stops at 98–99% for ever ("stuck at 99%"): it stretches a few of ProMods'
  village houses 130,000 times as long, and looking for bridges over a house 1,744 km long never ended. Only where
  there are bridges is looked at now; every other map comes out the same as before.
- Fixed the maps not being prepared after installing or updating WestPlay: WestPlay could start before the one the
  installer closed was fully gone, and stopped. It now waits for it.
- Fixed the Coaches DLC's coaches showing the game's navigation instead of WestPlay when the DLC was installed after
  WestPlay: WestPlay now sets its mod up again by itself, with the game closed, when a DLC with a WestPlay screen is
  installed or a game update changes one of the screens.
- Fixed WestPlay listing profiles the game doesn't have, with more than one Steam account on the PC: it read every
  account's Steam Cloud profiles, and now reads only the account signed in, as the game does. A map prepared for
  another account's profile is no longer kept.
- Fixed the game crashing on TruckersMP when a car or truck mod installed for single player has a dashboard at the
  same place as one of TruckersMP's own vehicles (a Škoda Superb mod and TruckersMP's Škoda Superb): WestPlay no
  longer copies any mod's dashboard into its mod, only the game's own.
- Fixed YouTube Music turning repeat back off: one press of repeat could go round all three modes to off.
- Fixed an app (Music on the MAN Lion's Coach's screen) pushing the whole screen up as it opened, a blank band below.

## 2.4.0 (2026-11-13)
- The Coaches DLC: WestPlay on the screens of the Scania Touring and the MAN Lion's Coach, shaped to each and drawn
  sharp. The game's parking camera shows over WestPlay, as on the game's own screen, and WestPlay under it takes no
  clicks while it shows. The Volvo 9700 keeps the game's navigation (its real one has no CarPlay); the WestPlay tablet
  in its dashboard shows WestPlay.
- The coaches play through their own speakers, from the moment you get in: the MAN Lion's Coach and the Scania
  Touring (both lengths each) and the Volvo 9700. The driver's pair, or every speaker over the passengers' seats,
  heard from the driver's seat: a choice in the companion (Truck Screen › Coach speakers), greyed out in a truck. In a
  coach it says how many speakers that coach's passengers' set plays at once, which a weak processor can feel.
- The screen's brightness on keys: Alt+4 darker, Alt+5 brighter (with WestPlay's key), shown at the top of the screen
  as the volume is. Both can be changed in the companion's Controls; a key you had already put another keybind on
  stays yours.
- Holding Volume up or Volume down changes the song after one second (it took two).
- The sidebar laid out as CarPlay's: its apps in the very middle of it on every screen, the clock at the top, the truck
  and Home/Dashboard buttons together at the bottom (a gap of most of a button stood between the two). Its buttons are
  larger, about half an app icon as on CarPlay (the tablet's stay as they were). The Scania 7" shows three apps (it
  had two).
- The map's speed limit sign keeps its figures inside its red ring at every Text Size, three-digit limits too (they
  grew with the Text Size and ran through the ring).
- Swiping the Home screen's pages no longer sticks when the mouse is let go at the screen's edge.
- Maps (and the Dashboard's map) move smoothly at 60 frames a second on every display: on displays faster than 60 Hz
  the game was handed as few as 45 new map frames a second (36 at 144 Hz), the map holding still up to 40 ms at a
  time, "moving like steps" whatever the graphics card. The map is now drawn evenly, so each of the 60 frames a second
  the game takes is a new one.
- Through every toll plaza and border crossing the route runs through the booth or gate the game's route drives
  through, wherever it stands: on many it ran straight through between the booths (south of Innsbruck), and where
  a booth or gate stood off the plaza's middle, beside it. On ETS2's map the line now passes through the booth on
  all 449 ways through toll plazas (252 before) and the gate on all 181 through border crossings (95 before).
- The game no longer crashes a while after it starts when the Steam overlay's code is in it (even switched off in Steam)
  and something makes another swap chain, as Trucky's app does as it connects: where the overlay had hooked the
  game's picture before WestPlay, the two called each other round until the game's stack ran out. WestPlay now leaves
  the overlay nothing to hook again.
- WestPlay starts its cab's browser again if it doesn't start the first time (up to three tries), instead of stopping
  with "WestPlay engine stopped: … the timeout period expired". When it does stop, app.log now says why.
- Every map is prepared again once with this update.
- A map with a map mod made of older models (Oyuncuyusbis map) no longer fails to prepare: an older model's parts
  were read as the game's current ones, and the build asked for 13 GB at once. Models in the game's three older
  formats are now read as they are, so their buildings, stations and roads are on the map too (on the Oyuncuyusbis
  map, 702 of its 4,523 models).
- Map mods whose definitions are written scrambled, encrypted or binary (as the game reads them) are read: their
  roads' looks, junctions and water were left out without a word. Whatever still can't be read is named in app.log.
- A map mod with damaged map files is left out and named, instead of the whole map failing to prepare or the
  preparation sitting still (a zip cut short, damaged sectors, junctions or models, a road of impossible length,
  ground placed nowhere; where a mod's sector doesn't fit the game's own, that mod is left out). A damaged or
  protected archive can no longer make a preparation ask for gigabytes at once ("memory allocation of … bytes
  failed").
- A map whose parts lie far apart (map mods across the world's oceans, or a DLC far from the rest: one player's
  spans 1,561 x 1,389 km) no longer needs tens of gigabytes of memory, or hundreds of thousands of files of open
  sea on disk, and its sea comes out right. Small lakes and islands far from the map's middle are drawn the right
  way (a lake could come out as land, an island as a hole).
- Prepare Maps Faster no longer runs out of memory on PCs where other programs use a good part of it (a laptop of
  16 GB failed with "memory allocation ... failed"): it now counts everything the PC uses. Preparing a map also
  takes less memory and less time on its own.
- Starting the game while a map is being prepared stops the preparation, and the companion says so: it went on
  under the game, taking the PC's memory and processor from it. It starts again from the beginning once the game
  is closed.
- The map bar moves the whole way through, at the pace the work really goes ("stuck at 92%" for minutes), and gives
  each map an equal share however many profiles play it.
- A map that fails ends with what went wrong, and the next profile's map is prepared: it could sit still at a
  percentage for ever. A failed map says the fault itself, not "a scoped thread panicked", and one that couldn't
  get the memory it needed says how much it asked for.
- A preparation that's stopped or fails no longer leaves gigabytes of unfinished files behind.
- Maps prepared by an older WestPlay are prepared again when the map builder changes: a map prepared by 2.2.0 that
  left no mod out was taken as up to date, so ATS's start at San Francisco (2.2.1) never reached it.

## 2.3.0 (2026-11-11)
- Safari: the web in the cab, in iPad Safari's look (iPadOS 26). Tabs in a tab bar of their own, or Compact, in the
  toolbar (Settings › Safari › Tabs); the bars take the colour of the page's top as it changes; the tab overview,
  History, the Start Page with your most visited sites, and the page menu's text size. What you type is a website or
  a search with the search engine you pick (Google, Yahoo, Bing, DuckDuckGo or Ecosia). Every website opens as its
  mobile site, laid out at Safari's width, and follows WestPlay's light and dark appearance. Sites keep you signed in
  (Google as YouTube and YouTube Music are), your open tabs and history are there next time, and Clear History clears
  the last hour, today, today and yesterday or all of it. Drag a page to scroll it as a finger does; a website's
  drop-down list opens in Safari's own menu; the cab's keyboard and your own type into a site's fields. A page's
  sound plays through the truck's speakers and pauses with the game, as the music apps do. Cloudflare's and other
  sites' "are you human" checks pass. No downloads, and nothing a page asks for (camera, location, notifications).
- Brightness, in Settings › Appearance: a slider between a small and a large sun, as iOS's. It turns down the light
  the screen gives off, the whole screen together (a video playing and the cursor too), not a dark layer over it.
- Through a toll plaza the route runs where the game sends a truck: through the trucks' booth, often off to the
  right, not straight through the cars' booths. Each plaza says which booths trucks may use, per direction, so some
  routes still go straight through. A weigh station's scale is passed by where there's another way (the plazas at
  Edirne and Istanbul): it calls a truck in only now and then, and you then drive off the route into it.
- Home has two rows of apps at most, as CarPlay's, the rest on the next page (the Scania Smart Dash had three). Its
  pages always move on to the page a drag settles on, from wherever it's let go: let go part of the way, they jumped.
- The sidebar's Dashboard button is CarPlay's: one rounded square, as large as the Home button's app grid. The
  sidebar's fourth app (on screens with room for it) is Calendar, Weather, Settings or Safari, whichever you used
  last.
- The tablet: no truck button (it's no truck's screen, so there was no way back), its sidebar's app icons a quarter
  larger and its Home button twice the size (they were smaller than a finger), and its screen less bright by day
  (it was far too bright).
- The cursor goes dark over anything light and light over anything dark, from what's on the screen under it: a web
  page, a video, a picture, the truck's own screen (it went by what the app said, and was often wrong).
- The keyboard's corners follow the screen's, and it rises over what's on the screen and slides away smoothly:
  nothing moves or changes size for it (a field low on the page scrolls up into sight). Typing on your own keyboard
  no longer pops each letter up over its key.
- A video playing on the Dashboard goes with its card as the map grows over the widgets (its picture stayed over
  the map), and shows again once the card is back.
- A map that fails to prepare says why: the companion showed only Rust's "note: run with `RUST_BACKTRACE=1`…" line,
  never the reason above it.
- WestPlay works with OptiScaler, Snowy Moon's TAA/DLSS and lighting mods, RivaTuner, the Steam overlay and other
  tools that wrap or hook the game's picture: the game crashed on its first frame with OptiScaler, and with Snowy
  Moon's mods or RivaTuner the truck's screen stayed black. WestPlay finds what to hook with a small program of its
  own outside the game (westplay-probe.exe, beside the plugin), hooks in front of those tools without changing any of
  their code, and hooks again should another tool write over its hook (RivaTuner's overlay still shows).
- An American Truck Simulator folder with ETS2's program copied into it is taken for ATS: setup didn't show it, and
  chosen by hand it was set up as ETS2.
- When WestPlay's engine starts again (the cab moved to another graphics card), what the one before ran stops with
  it: the same map was prepared twice at once, and the game's weather and route read twice.
- When the game crashes, plugin.log says which error ended it and the calls that led to it, and whose code each of
  WestPlay's hooks went into: a crash with an upscaler or another tool in the game can now be found from the log.

## 2.2.2 (2026-11-06)
- Prepare Maps Faster, for strong PCs: a switch on the companion's Map page, and a step of its own before WestPlay
  is installed, off unless you turn it on (and tick that you've read what it does). It prepares the maps with every
  thread of the processor and up to four fifths of the memory, in a map builder of its own with a faster way of
  taking memory: ETS2's map in about two minutes on a 16-core PC, where it took over five.
- Each map is prepared in a process of its own: a map that fails to prepare no longer takes WestPlay down with it.
- No map stops preparing for the memory it takes any more: a very large map mod (or several) stopped at three
  fifths of the PC's memory (19.2 GB with 32 GB) and was left out.
- Prepare Maps Again asks which maps to prepare: all of them, or the ones you pick.
- Preparing a map is faster on every PC, and takes less memory at the most: the buildings are placed on every
  thread, and a junction shaped the same as one already worked out isn't worked out again.
- Tablet: the music is no longer muffled when you look away from the tablet in the newer American trucks'
  interiors (the Volvo VNL 2025's, the Kenworth T680 2022's, the Freightliner Cascadia 2024's), nor when a mod
  changing a truck's dashboard is installed but active in none of your profiles.
- The Dashboard's map grows and shrinks smoothly: it moved at a stutter, the map drawn again at a new size each
  step of the way.
- When WestPlay or the game's plugin crashes, its log says what happened and where, and app.log keeps the run
  before as app.old.log and starts with WestPlay's version; setting WestPlay up writes each of its steps there:
  a crash on a player's PC can now be found from their logs.

## 2.2.1 (2026-11-05)
- Preparing a map takes less than half the memory it did, the same map coming out (ETS2's about 5.5 GB at the most
  where it was 12, ATS's under 3 where it was over 7): the roads' and junctions' shapes were kept three times over
  while it was drawn, the parts in the air copied once more for each piece of the map, and the densest pieces (the
  cities) all drawn at once. It takes a little longer.
- With no truck to follow (the companion's screen, the cab before you drive), ATS's map starts at San Francisco: it
  started in the middle of the states you own, which can be empty desert, and the companion's screen showed nothing.
  Every map is prepared again once after the update.
- Adding an icon set or wallpapers in the companion reads only the files it takes, and none past its size limit: a
  folder or a .zip with other big files in it (a video, a zip that unpacks to gigabytes) made WestPlay read them all
  into memory first (reported by kocaemre).

## 2.2.0 (2026-11-04)
- WestPlay comes to American Truck Simulator, next to Euro Truck Simulator 2: one WestPlay for both. The companion
  finds both games and sets WestPlay up in each in one go; its setup, map and update steps show each game. In ATS,
  WestPlay is on the tablet in every truck; the trucks whose real counterparts have CarPlay get WestPlay on their own
  screens later. ATS's map has only the states you own, its landmarks (the Strip's casinos, the Golden Gate, St
  Louis's arch, the downtowns), and zoomed out it goes on into the game's own picture of North America. Go, Add
  Stop and alternative routes work on ATS 1.62, and WestPlay finds the game's GPS in the game's own code, so an
  update that moves it doesn't break them.
- New on Home: Player, for the songs you put in the game's own music folder (Documents › Euro Truck Simulator 2 ›
  music, or American Truck Simulator's): all of them in one list, Play and Shuffle, Now Playing with shuffle,
  repeat and a volume bar, played through the truck's speakers as the other music apps are. A song without tags
  shows its file's name, and songs added to the folder show up as you add them. Player takes the sidebar's music
  place when you use it, shows on the Dashboard's Now Playing, and pauses with the game and when another app plays.
- WestPlay speaks your language on the truck's screen: the cab is in all 43 of the game's languages, not only
  English and Turkish, each saying things the way its players do, with the game's own words for the truck and its
  places. YouTube and YouTube Music answer in it too, and Apple Music names its Library's sorts in it. You can
  choose WestPlay's language in the companion (Setup › Language), over the game's own; place names on the map stay
  in the game's language. The companion's translations are written anew in every language.
- The on-screen keyboard has each language's own letters, laid out as on the iPhone: German's QWERTZ, French's
  AZERTY, the Nordic, Baltic and Central European letters, Greek, Cyrillic and Georgian. Korean's letters join into
  syllables as you type, and Japanese is typed in romaji that turns into kana. The globe key switches to English and
  back. Chinese and Vietnamese type Latin letters for now.
- Speeds and distances follow your profile's units (Options › Regional settings): with miles, the speed, the speed
  limit, the trip, turn-by-turn and search show mph, miles and feet.
- CarPlay's screen has its black rounded corners, as in a real car, round the sidebar at the same gap all the way, on
  every truck's screen. The Dashboard's widgets and map, the apps' windows and the keyboard round their corners as
  the sidebar does, the margin round CarPlay is smaller and the same on every screen, and the apps keep their buttons
  clear of the corners. The cursor goes under the corners, never over them.
- WestPlay fits each truck's screen as it really is, laid out to its shape instead of stretched onto it: it was drawn
  8% too narrow on the Scania Smart Dash, and a little out of shape on the Smart Dash Advanced, Volvo FH, Iveco S-Way
  and the tablet. Nothing is cut off at the screen's edges any more on the DAF, Iveco, Renault T and Volvo FH, and
  the bottom of the screen is no longer smeared. The coaches' screens are shown in their exact shape too.
- The sidebar works as CarPlay's: one app of each kind, the one you used last: Maps, then your music app, then
  Phone or Messages, and the Calendar where the screen has room.
- New on Home: Calendar, in Apple's look, its icon showing the day; and Phone and Messages, which aren't ready yet
  and say so.
- Driving at 30 km/h or more without touching the screen for 30 seconds, the Dashboard's map grows to fill it, the
  widgets sliding away; touch the screen or slow down and they come back. It can be turned off in Settings ›
  Dashboard.
- Hold Volume up for two seconds to skip to the next song, Volume down to go back. A short press still changes the
  volume.
- WestPlay's keys (Alt+1, Alt+2, Alt+3 or your own) are kept from the game from the moment you press them until you
  let go, also where the game reads the keyboard through DirectInput: there, Alt+1 and Alt+2 also switched the
  game's camera out of the cab.
- A new map, drawn from the game's own roads and junctions: intersections and bridges stand at their real heights,
  flyovers and interchange ramps rising over the roads under them and bridges over the sea and wide rivers as high
  as they are in the game, the route, your truck and the destination flag going up with them. The 3D buildings have
  their real shapes (pitched roofs, towers, domes, a stadium's bowl), from the game's own models. The companion's
  Map page has switches for the 3D roads and the 3D buildings; with the buildings off they lie flat in their shapes.
  The map is prepared again once, and takes more room.
- The map keeps its roads and yards as you zoom out, the streets the game leaves off its own map included, with a
  no-entry sign at their entrance. A filling station shows its road and forecourt, a tunnel stays a tunnel through
  its junctions, and the grass round junctions no longer shows as road.
- The route line runs on your side of the road: on any road without a wide central reservation it ran down the
  middle, and took your truck's mark there with it.
- Maps' search opens without the keyboard, its panel reaching down the map; the keyboard comes up when you press the
  search field, and the panel stops just above it on every screen. Find nearby's labels keep long words whole on
  small screens.
- On TruckersMP, WestPlay finds the server you're really on: it took most players to be on Simulation 1, its other
  players and traffic with it. On TruckersMP's ProMods servers the map is ProMods', kept beside the other servers'
  map. The TruckersMP map is no longer prepared again every time you quit the game.
- WestPlay shows on Oyuncuyusbis' coaches' screens again: their new versions scramble their files, which WestPlay
  couldn't read.
- The game could crash as it started, while WestPlay set itself up in it: WestPlay read a graphics object it had
  just let go of.
- Preparing a map takes half of your processor, never all of it, and the companion holds still behind the game on
  every PC, taking no processor time from it.
- The setup's last screen has its third step again: hitting the road, after starting the game and getting in the
  truck.
- The companion's YouTube and Discord buttons have Patreon beside them.

## 2.1.4 (2026-10-28)
- The cursor on WestPlay's screen stays where you left it when you let go of the interaction key, and fades away
  five seconds after you last moved it, as it does when you stop moving it: it went the moment you let go.
- The tablet stays blank in a truck whose own screen shows WestPlay (a Scania S with the tablet fitted): with both
  showing, WestPlay was laid out for one screen and then the other on every frame, and slowed down until it froze.
- YouTube Music plays songs YouTube shows a warning before ("this video may contain disturbing images"), as
  YouTube already does: the song stood still at its start behind the warning, which had no one to press continue.
- YouTube Music gets past YouTube's "Before you continue" cookie page in the EU, the UK and Switzerland: it could
  stay on it, nothing loading. WestPlay answers it as Reject all does, in YouTube too.
- Over a light album cover, Now Playing's words go dark, in Apple Music and in the Dashboard's widget, the
  sidebar's too: they stayed white on the light colours.
- The cab's sidebar is almost a pill again, its corners round, with more room at its top and bottom so the clock
  stays clear of its round ends: 2.1.3 left it nearly square.
- Music keeps playing while the game shows, with another window in front of it or not: with Lossless Scaling
  (its window is in front of the game it scales) YouTube, YouTube Music, Spotify and Apple Music paused as if
  you had tabbed out. It pauses as the game is minimised, and with the game's menu, as before. WestPlay's
  keybinds (volume, play/pause) work there too: they went dead with another window in front.
- With the camera outside the truck, music no longer plays for a moment as if you were in the cab while you
  go between the game's menus: WestPlay took the cab, drawn again behind a menu, for the menu gone and the
  camera back inside. The music now waits for the game to run again.
- With the game run as administrator, WestPlay says so: the companion's Truck Screen page, and Maps when you
  start a route ("The game didn't answer" before). WestPlay can't see the game's route, guidance or weather then;
  start the game normally, not as administrator.

## 2.1.3 (2026-10-27)
- The WestPlay tablet has a new look: a slim aluminium tablet with a black glass front, at a real tablet's size
  (24.8 x 17.9 cm), WestPlay laid out in the shape of its screen. The tablet's model is Art_bor's (Sketchfab,
  CC BY 4.0).
- Hanging, the tablet is held by a clamp over its top edge, on a short, sturdy arm: the long thin bar it hung
  from is gone.
- On the tablet, WestPlay uses more of the screen: the margin round it is smaller, as the tablet shows all of
  WestPlay's picture.
- Every panel's corners are rounded the same, as the Dashboard's widgets are: the sidebar, the map, the apps'
  windows and the keyboard each had their own.
- The truck's screen no longer freezes on its last picture after the game runs again from its menu: in some
  players' games, WestPlay stopped seeing the screen being drawn and left it as it was until the next pause.
- YouTube's videos fill the player: a video could play small in its top left corner, most often the next one
  after the first.
- Maps takes less of your graphics card and processor, most of all in towns with their 3D buildings: it was
  drawn as often as your monitor refreshes (144 times a second or more on some), though the game shows at most 60
  of WestPlay's frames. It's drawn at most 60 times a second now.
- In a truck with the tablet, the companion says WestPlay is on the truck's screen: it kept saying it was waiting
  for the cab, and its switch for WestPlay's screen stayed greyed out.
- The companion's window holds still while it isn't in front: its floating screen kept drawing behind the game,
  taking up to two processor threads the game could use. It goes on the moment you click it.
- The cursor on WestPlay's screen follows the mouse at once: it was drawn by the screen itself and showed three or
  four frames late, more at a low frame rate. WestPlay now draws it straight into the frame the game shows, the
  same dot as before, in a few milliseconds.
- WestPlay's log says more when its screen is slow: every 10 seconds instead of every minute, the screen's own
  frame rate beside the frames Windows delivers, the companion window's share of the processor, and the game's
  window (fullscreen, borderless or a window, and when it's in the background).
- The companion says when YMN One, another CarPlay mod, is installed in the game: it takes over the truck's GPS
  screen, so WestPlay can't show there, and the screen turns pink while YMN One isn't running. It says which file
  to delete.
- YouTube videos no longer stop playing altogether with "This content isn't available, try again later": the way
  WestPlay asks for a video without the ad's wait sometimes got that answer, and was kept for every video after it.
  That way is now dropped for the video asked again YouTube's own way, so it plays.
- YouTube and YouTube Music work in the EU, the UK and Switzerland: YouTube's "Before you continue to YouTube"
  cookie page stood in for them, with no one to answer it. WestPlay answers it for you as Reject all does.
- YouTube plays videos it shows a warning before ("this video may contain disturbing images"), such as some songs'
  videos: WestPlay's player stayed on the warning, which had no one to press continue.
- Spotify shows all your Liked Songs, and every song of a long playlist or album: the list stopped at the first
  50 (Liked Songs and albums) or 100 (playlists). The rest now load as you scroll down, as in Spotify.

## 2.1.2 (2026-10-26)
- On TruckersMP, your own truck no longer shows on the map as another player's dot following you.

## 2.1.1 (2026-10-26)
- Setup no longer says WebView2 isn't working on PCs where it is: 2.1.0's check looked in the wrong place in the
  registry, so it stopped every install and update.

## 2.1.0 (2026-10-26)
- WestPlay's plugin loads in the game on PCs without Microsoft's Visual C++ Redistributable: the game said
  "unable to load" (0000007E) in its game.log, and there was no WestPlay on the truck's screen while the app ran.
- Spotify pauses with the game and plays on as you come back, fading in, every time: leaving the game and
  coming back quickly could leave the music stopped, and a song could start late.
- Pausing with an outside camera no longer plays the music clear for a moment, as if from inside the cab: it
  stays muffled as it fades out.
- Text Size has two smaller steps, and Home's app names and Settings' titles follow it; the sidebar keeps its
  size, as in CarPlay.
- When a YouTube video doesn't start within 20 seconds, the log says why, as YouTube's page tells it: whether YouTube
  lets it play, its error, and what the video player has received. For players whose videos never start.
- The companion says when your truck isn't one WestPlay shows in ("This truck isn't supported"), instead of
  waiting for the cab, and that fitting the WestPlay tablet brings WestPlay into any truck.
- Setup checks that Microsoft Edge WebView2 Runtime works on your PC before installing: without it WestPlay
  couldn't start at all (it closed at once, showing nothing). Setup now says so, offers Microsoft's page to
  install it from, and stops.
- WestPlay in any truck: the WestPlay tablet, which you fit in the truck shop (Accessories: Hanging Items, or as
  a dashboard set), shows WestPlay in trucks with no screen of its own, the truck's own screen keeping the game's
  navigation. Hanging, it's on a short bar from the top of the dashboard, upright and level in every truck; on a
  dashboard set, it faces you. It also stands on the toy spots of a dashboard set you've fitted, such as a table's. As with the trucks' own screens, the music sounds from outside the cab as it would through its
  walls. It's in every truck with hanging items, mod trucks too: a truck mod you install later gets it the
  next time you start the game. The tablet's model is Kedi739's, used with his permission.
- Maps are prepared again for players with certain mods: a small packed file in a mod could stop WestPlay from
  preparing any map at all, leaving the setup's map step waiting for ever and Prepare Maps Again gone. A map
  that can't be made now fails on its own, with a reason, and the other profiles' maps are still made.
- YouTube Music starts at the volume you last set, instead of at full volume every time.
- Spotify no longer plays in silence once its volume is down to nothing: the volume bar disappeared at 0 (and
  with it the button to unmute), and the volume you last set was never given back, so a song seemed to play
  with no sound. The bar now stays, at 0, and Spotify is set back to your volume when it starts.
- More map mods show on the map: ones made with an older map editor (such as Sud de France and Bourges
  Updated), and ones protected in a way WestPlay couldn't read (such as Crni Kal). WestPlay had left them out
  of your map; a map that left a mod out is prepared again by itself, others are kept as they are.
- WestPlay's log (app.log) is always kept now. Where Windows' ransomware protection (Controlled folder access)
  or an antivirus keeps WestPlay out of Documents, there was no log at all; it's now kept in WestPlay's own
  folder, saying why, and About shows where it is. Starting WestPlay again while it was running also emptied
  the log; it no longer does.
- WestPlay on the screens of Oyuncuyusbis' coaches: the Mercedes-Benz Travego 16 SHD, the MAN Lion's Coach and the
  Setra S 531 DT on each one's own dashboard screen, and the Setra S 516 HD/2 on its tablet (its "Gps" part, fitted
  in the truck shop), each shaped to its screen and drawn sharp. Install the coach's mod and WestPlay sets its
  screen up by itself.
- Music plays from the speakers of JuseeTV's Scania PGR, as fitted in the truck shop: its door speakers, the
  A-pillars' tweeters, the top console's centre speaker (or Dark Diamond's pair), and the rear walls with
  speakers (JS Customs, V8 Power, Dark Diamond, Lights + speakers). A rear wall's biggest units play the bass;
  its smaller ones, where it has them, a quiet fill a moment after the front, so the music stays in front of
  you and the cab sounds bigger. Parts you change count once the game saves.
- The Renault T's speakers as each interior has them: the two speakers on the back wall over the bed in every
  T, the A-pillars' tweeters, and the door speakers only in the 2024 interiors. In the older interiors the music
  comes from the back wall, as in the truck. The low sleeper's speakers are now where its cab has them, no longer
  a high sleeper's, and right-hand-drive Ts have them mirrored. A truck you've just bought or changed counts once
  the game saves.
- The MAN TGX (2020)'s speakers: a woofer low in each door, a smaller unit above it for the voices, and a tweeter
  on each A-pillar, in every cab (GM, GN, GX), right-hand drive too.
- WestPlay on the screens of the DAF XF Electric and XD (the XF's screen, with its DAF strip) and the Scania S
  BEV (the S's 7" screen, drawn sharp as the S's is).
- The DAF XD's speakers: the XF and XG's, where its cab has them. The XF Electric has the XF's, and the Scania S
  BEV the S's, and the Renault E-Tech T the T 2024's.
- The Mercedes-Benz Actros (2014)'s speakers: a woofer low in each door, a unit high over each door for the voices,
  and a tweeter on each A-pillar, in every cab. The Actros (2009)'s: a woofer low in each door and a unit at each
  front corner of the overhead shelf. The Scania R (2009) and Streamline's: a woofer in each door, a tweeter in
  each A-pillar. The Volvo FH16 (2012) has the FH's, and the FH16 (2009) a speaker in each door. The DAF XF (105)'s
  as each cab has them: A-pillar tweeters, and two pairs on the overhead shelf in the Super Space Cab or one
  overhead speaker in the Space Cab and Space Cab Plus. The DAF XF Euro 6's: door woofers and A-pillar tweeters, with
  two pairs on the overhead shelf in the Super Space Cab or one overhead speaker in the other cabs. The MAN TGX's:
  door woofers, and in the XXL two small speakers high in each side wall. The MAN TGX Euro 6's: door woofers, and in
  the XL a small speaker in each roof corner. The Renault Magnum's: a speaker in each roof corner, and tweeters under
  the overhead console. The Renault Premium's: a speaker high in each side of the roof, as each roof has it.
  The Iveco Stralis's: a speaker in each end of the overhead console, and in the high roofs a small one high on the
  passenger's side; the Iveco Hi-Way's: the console's two, and one high on each side.

## 2.0.3 (2026-10-23)
- Spotify plays on when you come back to the game: leaving it and coming back could leave Spotify playing in
  silence, or quieter each time, its volume stuck part way through the fade.

## 2.0.2 (2026-10-23)
- WestPlay works on older versions of the game too, such as 1.58: 2.0.1 found what it needs in every version
  but there, where the game is built a little differently.
- The Companion's Map page shows its maps and its Prepare Maps Again button while WestPlay removes an old map
  no profile plays any more: removing a big one takes minutes, and until it was done the page had nothing to
  show and the button was gone.

## 2.0.1 (2026-10-23)
- WestPlay works on other versions of the game, not only the current Steam one: it now finds what it needs in
  the game itself, whatever its version. The VR version and older versions such as 1.60 included, and it should
  keep working through game updates.

## 2.0.0 (2026-10-22)
- TruckersMP: WestPlay works there, its screens brought into the game by its plugin as in single player.
  Maps uses TruckersMP's own map (its HQ and event cities), prepared as soon as TruckersMP is found.
- TruckersMP: the other players show on the map, live around you and further out every few seconds, and
  traffic on your route shows on the blue line in yellow and red where trucks are bunched up, as you choose
  a route and as you drive.
- Maps at night in Apple Maps' own dark colours: slate-blue ground, lighter buildings, deep blue water, teal
  parks, and roads as light lines further out and dark bands closer in.
- Maps by day in Apple Maps' own light colours: cream ground, white streets, grey motorways, sky-blue water;
  the 3D buildings in the colour of the flat ones, their walls softly shaded rather than near black.
- Maps shows the buildings in 3D as you drive, in their own shapes and heights from the game's models (a
  tower's pod, a roof over the pumps) and in the colour of the flat ones, as Apple Maps does. A filling
  station's or a border crossing's roof is left out while you're under it. Every map is prepared again once.
- Filling stations, garages, services, hotels, recruitment agencies and company depots stand in 3D too, their
  own buildings from the game's models, rather than lying flat.
- Warehouses and malls the game builds from walls stand as whole buildings, and the details on their roofs no
  longer stand in rows on the ground.
- Filling stations on Maps: no more white blobs where a station stands beside the road, and the roads through
  stations on the road join on at their own width instead of stopping short.
- One-way service roads in the Balkans join their forks and stations smoothly on Maps, without a step at each join.
- Tunnels on Maps: roads through tunnels are drawn as a faint band, as Apple Maps draws them.
- Text Size in Settings › Appearance, as on iPhone: seven steps from smaller to larger, for the text in every
  app and the names on the map, the icons and artwork as they are.
- An equalizer in Settings › Sound, as on iPhone: Recommended or Custom, Low, Mid and High dragged up or down
  (up to 6 dB each way) for every player and every truck, with the song playing and its waveform moving under
  the curve as you set it.
- The Volvo FH's own speakers: music plays from the woofers low in its doors, with the bass from the panels on
  its back wall.
- With head tracking, the truck's speakers no longer crackle as the tracker's small wobbles turn the sound:
  it follows your head only as it really moves.
- A truck mod is no longer taken for a map mod, so it doesn't get a map of its own prepared for nothing.
- The Companion's Map page shows one progress number in its bar and its sidebar, never running ahead of the
  map being prepared.
- WestPlay removes the maps no profile plays any more (a profile deleted, its map mods changed), freeing their
  space.
- Handing a route to the game's GPS: WestPlay's hidden waypoints go only on real road pieces, never on a
  junction or a border crossing's lanes, and at border crossings and toll plazas the game picks the lane trucks
  use. Before, the screen could flash several times and the game end up on its own way.
- Maps offers routes again where it found none, such as on a toll plaza's approach.
- Routes never go along roads no vehicle may drive.
- With the game's Time zones option on (Only time or Full info), the clock shows the time where you are, as
  the game's clock does: it was an hour early in Finland, the Baltics, Romania, Bulgaria, Greece and Turkey,
  and an hour late in the UK and Portugal. With it off, the clock stays the game's time. Every map is prepared
  again once.
- Pausing the game (its menu, its map) gives the mouse back to the game, also with "Press once instead of
  holding" on: the menu couldn't be used. Back in the game, press WestPlay's key again to use the screen.
- A stop you add goes where it fits along the way, as the game's own map puts one: from A to F, adding D and
  then B gives A, B, D, F. Without a job, a stop no longer becomes the end of the route.
- With stops on the route, the arrival pill shows the time to the next stop beside the time to the end
  ("30 min | 3 h 5 min"); once you reach it, the time to the one after.
- Closing the game's menu, the music comes back as the game's picture does, not 1.5 seconds later when the
  truck moves on.
- "Press once instead of holding" works: once on, pressing WestPlay's key again didn't turn it off.
- Apple Music's lyrics no longer pile up: each song's words stayed on the screen under the next song's. Spotify's
  home no longer risks showing a shelf twice or leaving one out when several share a title.
- Apple Music's synced lyrics follow every song: on songs whose times Apple writes in seconds, the last line had
  always shown as the one sung.
- Back in the game, the music fades in as it starts playing again, instead of staying silent for two seconds
  first (Spotify, Apple Music and YouTube Music).
- Apple Music's lyrics follow the song again, and a song from your Library no longer shows "couldn't load" over
  its own lyrics part way through.
- In Maps, pressing Go no longer makes the search panel jump from side to side while the game takes the route.
- Desktop icons can be clicked again while WestPlay runs: an invisible window of its truck screen covered the top
  left of the main screen and took the clicks there.
- WestPlay in the Volvo FH (2021 and 2024, their DLCs): the real FH has CarPlay with its INFOHIGH package.
- On the Scania S and R's 7" screen, WestPlay shows whole and sharp: its bottom was cut off, everything was
  drawn too tall, and the game drew the screen at a quarter of its resolution. Its sidebar shows two recent
  apps; the Dashboard's arrival time goes to the top of its map, clear of the speed and zoom buttons, and its
  next turn fits its column; Spotify's shortcuts show two across, with their names whole.
- A search box's hint too long for it shows its start, not its end. In Spotify, nothing shows under the player
  bar at the foot.
- The Dashboard's map has zoom buttons, as Maps does. They fade away after 15 seconds without using the map,
  and come back as you point at it.
- Keybinds: the music's volume up and down (5% a press) and play/pause, from the game. By default, hold
  WestPlay's key and press 1 (volume down), 2 (play/pause) or 3 (volume up); any of them can be any key, alone or with WestPlay's key, in the
  Companion's Controls. The volume shows at the top of the screen as it changes, as CarPlay shows it. A key
  bound to WestPlay no longer does anything in the game while it's WestPlay's.
- WestPlay's key can be any key now, not only the four offered, and it can be pressed once to use the screen
  and again to stop, instead of held (Controls, "Press once instead of holding").
- Leaving the game (its pause menu, the map, another window), the music fades out over half a second instead
  of cutting off, and fades back in as you return. Pausing it yourself still stops it at once.
- Spotify starts at the volume you last set, instead of its own every time (YouTube, YouTube Music and Apple
  Music already did).
- After switching from a truck WestPlay supports to one it doesn't, holding WestPlay's key no longer takes
  the mouse from the game.
- Maps shows the rest areas made without a rest area's own building block, where the game offers sleep by the
  road: over a thousand more on the base map, about 1,800 more with ProMods, as the game's own map shows them.
  Every map is prepared again once.
- WestPlay finds your game's folder when the game is started with `-homedir` in its Steam launch options, so
  it reads your own profiles and their mods (ProMods and others) instead of an old folder in Documents.
- In Light mode, the other routes Go offers are drawn pale, as Apple's are, instead of staying dark.
- Maps has a compass right over its zoom buttons, as CarPlay's Maps does: its red mark and N turn with the
  map, always pointing north. Tap it and the map turns north up, seen from above, still following your
  truck; the arrow then brings the view from behind the truck back.
- At toll plazas, the route on the map goes through a booth trucks may use, as the game's GPS sends you (often
  the right-hand ones), instead of straight through a booth for cars only.
- One player at a time, as on a phone: starting music or a video in Apple Music, Spotify, YouTube Music or
  YouTube pauses whichever of them was playing, instead of both playing over each other.
- When WestPlay offers an update, each change in its notes shows whole, instead of being cut off after its
  first line.
- A wallpaper of your own that's a single picture works in Light and Dark mode alike. It had kept the screen
  (and its icons) in one look, dark or light by the picture's brightness, with no way to switch.
- A map mod WestPlay can't read no longer stops a profile's map: the map is made without it, and the Map page
  names it on the profile. Mods whose archives give wrong sizes for their files (a trick of protected mods,
  which the game reads anyway) are read in full.
- On a route, Maps' search and Find nearby offer only places along the rest of the route, nearest along it
  first, with how far along the route each is and what stopping there adds ("+5 min"): no more stops
  behind you. Adding one says what it added, in the game's own distance and time.
- Go offers the routes to a place first, as Apple Maps does: the fastest, the shortest, one on small roads or
  another way, each with its time and distance, drawn on the map. The one you pick is handed to the game's
  GPS, which follows it, held to it by as few waypoints as it needs (up to nine, in the middle of roads); if
  it won't follow, it takes its own way, with none of them left behind. Every map is prepared again once, for
  its routes. The routes show whole, beside the card; a route can be picked on the map too, by tapping it; Go
  sits in the picked route's row. After Go, the map is back on the truck at the zoom it had before. Another
  way no longer leaves a motorway at an interchange only to come straight back onto it.
- WestPlay's truck screens no longer need turning on in Mod Manager, for each profile: WestPlay's plugin
  brings them into the game itself, in every profile and in Convoy, and they're not in your mod list at all.
  The old WestPlay mod is taken out of the game's mod folder and out of your profiles when WestPlay updates
  (with the game closed); uninstalling WestPlay still takes everything out.
- On the Scania Smart Dash Advanced (12.9"), WestPlay is laid out for its own screen: everything is smaller
  than on the 10.1", so it no longer looks too large, with text a little larger than the 10.1"'s so Maps'
  small print stays easy to read. WestPlay's
  screens mod now carries its own copies of those interiors' definitions to tell the two apart. Its taller
  sidebar shows five recent apps.
- Apple Music, Spotify and YouTube Music open smoothly the first time after the screen comes on: their window
  grows out of the icon showing a saved copy of how the app first looks (its own loading screens, signed
  in), and the app takes its place once open, its content loading as usual.
- Closing the game (or the game crashing) stops the music: Apple Music, Spotify, YouTube Music and YouTube
  are closed with it, instead of playing on, unseen, through the truck's speakers with no game. The screen
  starts afresh with the next game.
- When WestPlay's app and its game plugin are of different versions (one updated, the other not yet), they
  no longer try to work together: they had misread each other, and typing could take the truck's driving
  keys.

## 1.2.0 (2026-10-19)
- With map mods (ProMods, Türkiye maps and others), the route follows the mod's own junctions and the
  Maps app gives its turns there, instead of only where the mod left the game's roads as they were.
- WestPlay's screen is drawn on the same graphics card as the game, not on whichever card Windows lists
  first (the slow built-in one on many laptops). If it picks the wrong one, choose another under Truck
  Screen › Graphics Card. About shows WestPlay's card and the game's.
- ProMods maps show all their junctions: about 16,000 were missing, leaving roads cut short (most visible
  with ProMods Maghreb). ProMods players' maps are prepared again once, after the update.
- Maps with ProMods' Middle East add-on can be prepared: one of its roads was taken for a road 116 km away,
  and preparing the map ran out of memory.
- Preparing a map takes about half the memory (ProMods: 4 GB down to 2.2 GB) and runs faster on PCs with
  more cores. A map that would need more memory than the PC can spare is stopped, and the Map page says why.
- A filling station two fuel companies stand by is always named after the nearer one.
- Map mods are layered on the map as the game layers them: the one higher in the Mod Manager wins where two
  change the same thing (WestPlay had them the other way round). Every map is prepared again once.
- The Maps app shows the map of the profile you've loaded, as soon as you load it. It had followed the profile
  with the newest save, so a profile not saved for a while showed another profile's map (and garages).
- After Prepare Maps Again, the Map page's progress bar is all the maps being prepared, each profile an equal
  share (ten profiles: 10% each), instead of saying it's done once yours is. Each profile's row shows its own.

## 1.1.3 (2026-10-18)
- In a convoy, WestPlay's clock, day and night, weather times and arrival times follow the convoy's time,
  as the cab's own clock does, instead of your own game's.
- Preparing a map shows how far it is all the way through, not only at a few points.

## 1.1.2 (2026-10-18)
- ProMods 2.84's map can now be prepared: its older parts, which 1.1.1 still couldn't read, are read too
  (checked against ProMods' own map, all 1,247 of its parts).
- If the screen mod can't be installed, the message says which step failed and where.

## 1.1.1 (2026-10-18)
- Maps from map mods made with an older version of the game's editor, ProMods 2.84 among them, can be
  prepared. Water painted by those mods may be missing for now.
- WestPlay finds the game's folder when it's in OneDrive but Windows' Documents points elsewhere (or the
  other way round), so the screen mod installs and your profiles and maps are found.

## 1.1.0 (2026-10-18)
- WestPlay now appears only in the trucks it's made for: the DAF XF and XG 2021, the Scania S and R, the
  Iveco S-Way and the Renault T 2024. Other trucks (the Mercedes-Benz Actros MP4, say) keep their own screen,
  and holding the key no longer takes their mouse. More trucks come with the tablet.
- WestPlay prepares a map for every profile, one after another, starting with the one you play. A profile
  whose map can't be prepared no longer leaves your other profiles without one, and the Map page shows each
  profile's map: ready, being prepared, or why it couldn't be.
- The Map page has a Prepare Maps Again button: every profile's map is made anew, one by one, failed ones
  included. The cab keeps showing your map until the new one is ready.
- Maps by HVT are recognised whatever they're called, so WestPlay says they aren't allowed instead of failing
  to prepare the map.

## 1.0.4 (2026-10-18)
- Setup no longer counts the Feldbinder and Krone trailer packs as map DLCs.
- The tutorial at the end of setup reads better: rewritten in Turkish, and clearer in English and every
  other language.

## 1.0.3 (2026-10-18)
- Spotify no longer goes silent after a pause, or by itself between songs, when it plays through the truck's
  speakers.
- Uninstalling removes everything you asked it to, your maps and sign-ins included, and no window flashes up.
- Signing out of a service finishes properly, without the button spinning on.
- A truck from a mod (Jasper's DAF 2021, say) counts as installed even when no profile uses the mod yet.

## 1.0.2 (2026-10-17)
- You can join a friend's convoy, or have them join yours, when only one of you has WestPlay: its mod is
  marked as optional in convoys.

## 1.0.1 (2026-10-17)
- A new version of WestPlay is offered as soon as you open it, whichever page it opens on.
- If WestPlay can't check for updates (no internet, say), it tries again in half an hour instead of six hours.

## 1.0.0 (2026-10-17)
- First release.
