# Changelog / Изменения

## 1.1.6 — 2026-10-27

### English

- Added a Capacity / Speed switch for disk circles, with per-letter read and write rates from Windows LogicalDisk counters sampled once per second.
- Kept occupied space on the outer ring. Speed mode displays separate read/write lines and animates combined throughput on the inner ring against an adjustable decimal-MB/s scale, initially 1000 MB/s.
- Preserved the capacity view and individual drive visibility in existing profiles. Unknown readings remain unavailable rather than becoming zero.

### Русский

- Добавлен переключатель «Объём / Скорость» для кругов дисков. Скорости чтения и записи по буквам поступают из счётчиков Windows LogicalDisk раз в секунду.
- Внешнее кольцо сохраняет заполненность диска. В режиме скорости показаны отдельные строки чтения и записи, а внутреннее кольцо плавно показывает их сумму относительно настраиваемой шкалы, по умолчанию 1000 МБ/с.
- Старые профили сохраняют режим объёма и видимость отдельных дисков. Недоступные показания не подменяются нулём.

## 1.1.5 — 2026-10-27

### English

- Added optional used/total dedicated GPU memory inside the existing GPU circle. With load and temperature enabled, the temperature shares the header and memory occupies the lower line.
- Read memory from the same NVIDIA adapter through optional NVML v2 with a v1 fallback, once per second. Missing data stays unavailable; shared system RAM is excluded. Driver reserve accounting can differ from Windows.
- Kept compact text within the rings at 80–220 px and added a dedicated memory switch and detailed tooltip.

### Русский

- Добавлена отключаемая строка занятой и общей видеопамяти в существующий круг ГП. При включённых нагрузке и температуре температура размещается рядом с подписью ГП, память — нижней строкой.
- Видеопамять читается у того же адаптера NVIDIA через NVML v2 с резервным v1 раз в секунду. Недоступный датчик остаётся недоступным; общая память из ОЗУ не включается. Учёт резерва драйвера может отличаться от Windows.
- Текст укладывается внутри колец при размере 80–220 px; добавлены отдельный переключатель памяти и подробная подсказка.

## 1.1.4 — 2026-10-27

### English

- Restored the notification-area icon after Explorer restarts, including elevated startup, and retry registration during delayed shell startup.
- Kept widgets, settings and hover windows off the taskbar after shell restarts.
- GPU load now prefers Windows WDDM counters for the same uniquely identified NVIDIA adapter. Process counters are added within each engine, then the busiest engine is used, following Task Manager's aggregation. NVIDIA remains the temperature source and a labelled load fallback when Windows counters are unavailable.
- The GPU tooltip identifies the active load source. Genuine readings are not filtered to conceal spikes.

### Русский

- Значок трея восстанавливается после перезапуска Проводника, в том числе при повышенном автозапуске; добавлены повторные попытки при задержке запуска оболочки.
- Виджеты, настройки и подсказки остаются вне панели задач после перезапуска оболочки.
- Для нагрузки ГП предпочтительно используются счётчики Windows WDDM того же однозначно определённого адаптера NVIDIA. Показания процессов складываются внутри каждого движка, затем выбирается самый занятый, как в диспетчере задач. NVIDIA остаётся источником температуры и явно обозначенным резервным источником нагрузки при недоступности системных счётчиков.
- В подсказке ГП указан действующий источник нагрузки. Фильтры для скрытия пиков не применяются.

## 1.1.3 — 2026-10-26

### English

- Settings are flushed to a temporary file and replaced atomically, with a validated last-good backup used for recovery. Recovering a profile also preserves the selected monitor.
- Continuous edits no longer postpone saving indefinitely, and temporary save failures are retried.

### Русский

- Настройки записываются с атомарной заменой файла и резервной копией для восстановления. При восстановлении сохраняется выбранный монитор.
- Непрерывное изменение настроек больше не откладывает запись бесконечно; при временной ошибке программа повторяет сохранение.

## 1.1.2 — 2026-10-25

### English

- Statically linked the MSVC C runtime in the main Rust executable, removing its dependency on a separately installed Visual C++ runtime. This fixes launch failures on Windows systems without `VCRUNTIME140.dll`.
- Added a release check that rejects external compiler runtime DLL imports in the main executable before packaging is accepted.
- The sensor helper still requires .NET Framework, and direct CPU sensor access may require PawnIO and administrator rights.

### Русский

- Основной исполняемый файл на Rust теперь статически связывается с C runtime MSVC и не требует отдельно установленного Visual C++ runtime. Исправлен запуск на системах Windows, где отсутствует `VCRUNTIME140.dll`.
- Добавлена проверка релиза, которая отклоняет основной исполняемый файл с импортом внешних DLL среды выполнения компилятора.
- Модулю датчиков по-прежнему нужен .NET Framework; для прямого чтения датчиков ЦП могут потребоваться PawnIO и права администратора.

## 1.1.1 — 2026-10-25

First public release of Orbit Widgets. This version includes the features developed before publication and the tooltip fix below.

### English

- Added transparent circular widgets for CPU/GPU load and temperature, used RAM, occupied drive space by letter, and network receive/send traffic.
- Added the dual-ring layout, individual metric and drive switches, Mbit/s and MB/s network units, adapter selection and an adjustable network scale.
- Added vertical/horizontal layouts, automatic wrapping, circle size and spacing, opacity, four themes, monitor selection and edge/corner anchors. New profiles default to the right-center of a secondary monitor when available.
- Added free dragging, optional always-on-top placement, click-through locking, notification-area controls and global shortcuts.
- Added current-user startup at sign-in with an optional administrator mode through Windows Task Scheduler.
- Added NVIDIA NVML monitoring, a LibreHardwareMonitor sensor helper, existing MSI Afterburner shared-memory CPU temperature readings, and an installer option for the signed PawnIO CPU sensor driver.
- Fixed clipped hover details when the circles are arranged vertically at the right screen edge. Details now use a separate native window, wrap long text and stay within the monitor work area, including monitors with negative desktop coordinates. Initialized Windows layered-window opacity so the click-through tooltip remains visible.
- Provided a Windows x64 installer and portable package with third-party notices and SHA-256 checksums.

### Русский

Первый публичный выпуск Orbit Widgets. В него входят функции, подготовленные до публикации, и исправление подсказок.

- Добавлены прозрачные круги с нагрузкой и температурой ЦП/ГП, занятой оперативной памятью, занятым местом на дисках по буквам, приёмом и отдачей сети.
- Добавлены оформление «Двойной контур», отдельные переключатели показателей и дисков, единицы сети Мбит/с и МБ/с, выбор адаптера и максимума сетевой шкалы.
- Добавлены вертикальное и горизонтальное размещение, автоматический перенос, настройка размера и расстояния, непрозрачность, четыре темы, выбор монитора и привязка к краям и углам. Новый профиль по умолчанию размещается справа по центру дополнительного монитора, если он подключён.
- Добавлены свободное перемещение, режим поверх других окон, закрепление с пропуском кликов, значок в области уведомлений и глобальные сочетания клавиш.
- Добавлен автозапуск текущего пользователя при входе в Windows с необязательным запуском от администратора через Планировщик заданий.
- Добавлены мониторинг NVIDIA через NVML, модуль LibreHardwareMonitor, чтение температуры ЦП из общей памяти уже запущенного MSI Afterburner и установка подписанного драйвера датчиков PawnIO по выбору в установщике.
- Исправлено обрезание подсказок при вертикальном расположении кругов у правого края. Подробности открываются отдельным окном с переносом текста в границах рабочей области монитора, в том числе на экранах с отрицательными координатами. Добавлена инициализация прозрачности Windows, необходимая для видимости окна с пропуском кликов.
- Подготовлены установщик и переносной пакет для Windows x64 с лицензиями сторонних компонентов и контрольными суммами SHA-256.
