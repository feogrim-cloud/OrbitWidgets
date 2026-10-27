# Changelog / Изменения

## 1.1.10 — 2026-10-27

### English

- Set CPU/GPU load and temperature, video memory, RAM, network and drive read/write sampling to once per second. Drive capacity and occupied-space discovery remain on their five-second interval.
- Kept the short, adaptive ring animations independent of the one-second readings. Existing settings and the recovery and security fixes remain unchanged.

### Русский

- Нагрузка и температуры ЦП/ГП, видеопамять, ОЗУ, сеть, скорости чтения и записи дисков обновляются раз в секунду. Опрос объёма и занятого места дисков сохраняет интервал пять секунд.
- Короткая адаптивная анимация дуг работает независимо от секундного обновления показаний. Существующие настройки и исправления восстановления и безопасности сохранены.

## 1.1.9 — 2026-10-27

### English

- Smoothed ring changes with 200 ms transitions. Frames are requested at up to about 60 fps for larger movements and about 30 fps for movements below 4 physical pixels. Changes of 0.35 physical pixels or less snap to the target; without animation the repaint interval is 250 ms. Missing readings reset the affected arc immediately.
- Kept numeric readings and sensor sampling intervals unchanged. The animation changes only how the arcs move between received values; existing settings and the 1.1.8 recovery and security fixes are retained.

### Русский

- Дуги плавно переходят к новому значению за 200 мс. При больших перемещениях кадры запрашиваются с частотой до примерно 60 в секунду, при перемещениях меньше 4 физических пикселей — около 30. Изменения до 0,35 физического пикселя включительно сразу устанавливают целевое значение; без анимации интервал перерисовки составляет 250 мс. При недоступном показании соответствующая дуга сразу сбрасывается.
- Числовые показания и частота опроса датчиков не изменены. Меняется только движение дуг между полученными значениями; существующие настройки и исправления восстановления и безопасности из 1.1.8 сохранены.

## 1.1.8 — 2026-10-27

### English

- Cleared stale GPU temperatures and other values from circle headers and hover details as well as the main numbers. A disconnected source no longer leaves an apparently current value in a secondary label.
- Stopped reading cached sensor values when LibreHardwareMonitor hardware refresh fails; independent healthy hardware remains usable. GPU name, load and temperature now come from one coherent adapter, with missing or ambiguous requested adapters left unavailable.
- Added a ten-second watchdog for a helper that remains alive without valid output, bounded each output record to 16 KiB, and cleared malformed or oversized samples before recovery. A Windows Job Object also terminates the helper when the parent exits.
- Reopened failed NVIDIA NVML connections every thirty seconds and when **Обновить поиск датчиков** is selected, restoring GPU load, temperature and video memory after a transient initialization or device failure.
- Registered startup task XML directly in memory through Windows Task Scheduler COM. Elevated startup now requires a protected Program Files installation, checking ownership, ACLs, bundled dependencies, parent directories and reparse points before elevation or registration. Ordinary portable use and non-elevated portable startup remain supported.
- Resolved Windows system, Program Files and font paths through native Windows APIs instead of environment overrides. Restricted the optional Windows UI font read to the OS Fonts directory and a 16 MiB limit.
- Updated egui/eframe to 0.34.3 and its maintained skrifa/vello_cpu font stack, removing the unmaintained ttf-parser dependency. Kept the settings schema, display controls and existing profile values unchanged.

### Русский

- Устаревшие температуры и другие показания ГП удаляются из заголовка круга и подсказки вместе с основным числом. При потере источника дополнительная подпись больше не сохраняет видимость актуального значения.
- При ошибке обновления оборудования LibreHardwareMonitor сохранённые значения датчиков больше не используются; независимое исправное оборудование продолжает работать. Имя, нагрузка и температура ГП относятся к одному адаптеру; отсутствующий или неоднозначный запрошенный адаптер остаётся недоступным.
- Добавлен контроль модуля, который работает, но десять секунд не передаёт корректные данные. Одна строка ограничена 16 КиБ; некорректные и слишком большие данные очищаются перед восстановлением. Windows Job Object завершает модуль при выходе основного процесса.
- Подключение к NVIDIA NVML после сбоя повторяется раз в тридцать секунд и по кнопке **«Обновить поиск датчиков»**, восстанавливая нагрузку, температуру и видеопамять после временной ошибки инициализации или устройства.
- XML задания автозапуска передаётся Планировщику Windows через COM в памяти. Повышенный автозапуск требует защищённой установки в Program Files: до запроса прав или регистрации проверяются владелец, ACL, зависимости, родительские каталоги и перенаправляющие ссылки. Обычный запуск и обычный автозапуск переносной версии доступны.
- Системные пути Windows, Program Files и шрифтов определяются через Windows API вместо переопределений окружения. Чтение дополнительного системного шрифта ограничено папкой Fonts из системного пути и размером 16 МиБ.
- egui/eframe обновлён до 0.34.3 с поддерживаемым шрифтовым движком skrifa/vello_cpu; зависимость от неподдерживаемого ttf-parser удалена. Схема настроек, элементы управления и значения существующего профиля сохранены.

## 1.1.7 — 2026-10-27

### English

- Added occupied RAM percentage to the existing circle header and available physical memory to its bottom line, keeping used memory in the center and total capacity visible below it in the same circle.
- Used one consistent RAM snapshot every 500 ms for the numbers and occupied-memory ring. Available physical memory includes the reclaimable part of the Windows cache.
- Added a compact RAM gallery scene at 83 and 128 px.

### Русский

- В заголовок существующего круга ОЗУ добавлен процент занятой памяти, в нижнюю строку — доступная физическая память. Занятый объём остаётся в центре, общий объём — видимой строкой под ним в том же круге.
- Числа и кольцо заполнения используют единый снимок ОЗУ каждые 500 мс. Доступная физическая память включает возвращаемую часть кэша Windows.
- Добавлена иллюстрация круга ОЗУ при размере 83 и 128 px.

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
