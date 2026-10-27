# Orbit Widgets v1.1.10

## 1.1.10 — One-second readings / Показания раз в секунду

**English:** CPU/GPU load and temperature, video memory, RAM, network and drive read/write speeds now update once per second. Drive capacity and occupied-space discovery remain at five seconds. Short, adaptive ring animations still run independently of sensor sampling. Existing settings and all previous recovery and security fixes are retained.

**Русский:** нагрузка и температуры ЦП/ГП, видеопамять, ОЗУ, сеть, скорости чтения и записи дисков теперь обновляются раз в секунду. Опрос объёма и занятого места дисков остаётся раз в пять секунд. Короткая адаптивная анимация дуг работает независимо от опроса датчиков. Существующие настройки и все прежние исправления восстановления и безопасности сохранены.

## 1.1.9 — Smoother rings / Плавные дуги

**English:** ring changes use 200 ms transitions. While the arcs move, frames are requested at up to about 60 fps for larger movements or about 30 fps below 4 physical pixels. Changes of 0.35 physical pixels or less snap to the target; without animation the repaint interval is 250 ms. Unavailable readings reset their arcs immediately. Numeric values and sensor polling intervals are unchanged. Existing settings and all recovery and security changes from 1.1.8 are retained.

**Русский:** дуги переходят к новому значению за 200 мс. Во время движения кадры запрашиваются с частотой до примерно 60 в секунду при больших перемещениях или около 30 при перемещениях меньше 4 физических пикселей. Изменения до 0,35 физического пикселя включительно сразу устанавливают целевое значение; без анимации интервал перерисовки составляет 250 мс. Недоступные показания сразу сбрасывают свои дуги. Числа и частота опроса датчиков не изменены. Существующие настройки и все исправления восстановления и безопасности из 1.1.8 сохранены.

## 1.1.8 — Recovery and security / Восстановление и безопасность

**English:** this version keeps the existing settings schema and display controls. Metric switches, size, spacing, theme, opacity, monitor and placement remain in the current-user profile.

- Stale GPU values are cleared from the circle header and tooltip. Failed hardware refreshes cannot return cached readings, and fallback GPU name, load and temperature belong to one unambiguous adapter.
- A helper without valid output for ten seconds enters recovery. Records are limited to 16 KiB, malformed data clears its readings, and a Windows Job Object terminates the helper when the parent exits.
- NVIDIA NVML reconnects after failure at thirty-second intervals; **Обновить поиск датчиков** also reopens it immediately.
- Startup registration uses Task Scheduler COM with XML in memory. Elevated startup requires protected Program Files files and directories, verified ownership and ACLs, and no reparse points. Ordinary portable use and non-elevated startup remain available.
- Native Windows APIs resolve system and font paths. The GUI now uses egui/eframe 0.34.3 with skrifa/vello_cpu, removing the unmaintained ttf-parser dependency; the optional system font is bounded to 16 MiB.

**Русский:** схема настроек и элементы управления сохранены. Переключатели показателей, размер, расстояние, тема, непрозрачность, монитор и расположение остаются в профиле текущего пользователя.

- Устаревшие показания ГП удаляются также из заголовка круга и подсказки. Ошибка обновления датчика не возвращает старые значения; резервные имя, нагрузка и температура ГП относятся к одному однозначно выбранному адаптеру.
- После десяти секунд без корректных данных модуль датчиков переходит к восстановлению. Строки ограничены 16 КиБ, некорректные данные очищают показания, при выходе приложения модуль завершает Windows Job Object.
- Подключение к NVIDIA NVML после сбоя повторяется раз в тридцать секунд; **«Обновить поиск датчиков»** также сразу переподключает его.
- Автозапуск регистрируется через COM Планировщика с XML в памяти. Для повышенного режима нужна защищённая установка в Program Files с проверенными владельцем и ACL, без перенаправляющих ссылок. Обычный запуск и обычный автозапуск переносной копии доступны.
- Системные пути и шрифты определяются через Windows API. egui/eframe обновлён до 0.34.3 с skrifa/vello_cpu, удалена зависимость от неподдерживаемого ttf-parser; дополнительный системный шрифт ограничен 16 МиБ.

See [SECURITY.md](https://github.com/feogrim-cloud/OrbitWidgets/blob/v1.1.10/SECURITY.md) / Подробности: [SECURITY.md](https://github.com/feogrim-cloud/OrbitWidgets/blob/v1.1.10/SECURITY.md).

## Интерфейс / Interface

**1.1.7:** the RAM circle now shows occupied percentage in its header and available physical memory on its bottom line. Used memory remains in the center, and total capacity stays visible directly below it in the same circle. The tooltip also lists all capacities with units. All RAM values and the outer occupied-memory ring use one snapshot every 500 ms; available memory includes the reclaimable part of the Windows cache.

**1.1.7:** в заголовке круга ОЗУ добавлен процент занятой памяти, нижней строкой — доступная физическая память. Занятый объём остаётся в центре, общий объём — видимой строкой под ним в том же круге. Подсказка также перечисляет все объёмы с единицами. Все показания ОЗУ и внешнее кольцо заполнения используют единый снимок каждые 500 мс; доступная память включает возвращаемую часть кэша Windows.

[![ОЗУ: процент и доступная память, 83 и 128 px · RAM percentage and available memory at 83 and 128 px](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/ram-memory.png)](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/ram-memory.png)

**1.1.6:** disk circles now offer **Объём / Скорость** (Capacity / Speed). Read and write speeds are sampled per drive letter once per second from Windows LogicalDisk counters. The outer ring keeps occupied space; the inner ring in Speed mode shows combined read/write throughput against an adjustable scale, initially 1000 decimal MB/s. Existing profiles retain Capacity mode and their drive visibility.

**1.1.6:** в кругах дисков добавлен переключатель **«Объём / Скорость»**. Скорости чтения и записи по буквам поступают из счётчиков Windows LogicalDisk раз в секунду. Внешнее кольцо сохраняет заполненность; внутреннее в режиме скорости показывает сумму чтения и записи относительно настраиваемой шкалы, по умолчанию 1000 МБ/с. Старые профили сохраняют режим объёма и видимость дисков.

[![Объём и скорость диска при 83 и 128 px · Disk capacity and speed at 83 and 128 px](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/disk-speed.png)](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/disk-speed.png)

**1.1.5:** optional dedicated GPU memory (used / total) inside the GPU circle; temperature moves beside the GPU title when both load and memory are enabled. **1.1.5:** отключаемая строка занятой и общей видеопамяти внутри круга ГП; температура переносится к подписи, когда показаны нагрузка и память. The readings come from the same NVIDIA adapter. Показания относятся к тому же адаптеру NVIDIA.

[![Круг ГП с видеопамятью, 83 и 128 px · GPU memory at 83 and 128 px](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/gpu-memory.png)](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/gpu-memory.png)

Настоящий интерфейс с демонстрационными показаниями. The actual interface rendered with demonstration readings. Нейтральная подложка не является панелью приложения. The neutral backdrop is not an application panel.

[![Горизонтально · Horizontal layout](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/hero-horizontal.png)](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/hero-horizontal.png)

[![Светлая тема · Light theme](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/theme-light.png)](https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/theme-light.png)

<table>
  <tr><th>Вертикально · Vertical layout</th><th>Настройки · Settings</th></tr>
  <tr>
    <td align="center"><a href="https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/vertical.png"><img src="https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/vertical.png" height="420" alt="Вертикальные виджеты справа · Vertical widgets at the right edge"></a></td>
    <td align="center"><a href="https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/settings.png"><img src="https://raw.githubusercontent.com/feogrim-cloud/OrbitWidgets/main/assets/screenshots/settings.png" height="420" alt="Настройки · Settings"></a></td>
  </tr>
</table>

## English

Orbit Widgets provides transparent circular performance widgets for Windows x64. The application interface is in Russian.

**Tray and GPU monitoring fixes in v1.1.4:** the tray icon is restored after Explorer restarts, including elevated startup, and the widget windows remain off the taskbar. GPU load prefers Windows WDDM counters for the same uniquely identified NVIDIA adapter, using the busiest engine like Task Manager. NVIDIA remains the temperature source and a labelled load fallback. The tooltip identifies the load source.

- Monitor CPU/GPU load and temperature, used RAM, occupied drive space by letter, and network receive/send traffic in Mbit/s or MB/s.
- Combine load and temperature in the dual-ring design. Choose individual metrics, vertical or horizontal placement, size, spacing, opacity, four themes, monitor and screen anchor.
- Start a new profile vertically at the right-center of a secondary monitor when available. Lock the widgets to pass clicks through, open Settings with `Ctrl+Alt+O`, or toggle the lock with `Ctrl+Alt+L`.
- Use optional startup at sign-in, including an administrator mode through Windows Task Scheduler.
- Read NVIDIA GPU data through NVML, with a LibreHardwareMonitor sensor fallback where supported. CPU temperature can also use the fresh shared-memory reading of an existing MSI Afterburner instance. The installer offers the official signed PawnIO 2.2.0 driver for direct CPU sensor access.
- Fixed hover details being clipped by narrow vertical widgets at the right screen edge. Details now open in a separate visible window, wrap text and stay within the monitor work area, including negative desktop coordinates.

**Choose a download:**

| File | Purpose |
| --- | --- |
| `OrbitWidgets-Setup-x64.exe` | Windows x64 installer with optional CPU sensor driver installation. |
| `OrbitWidgets-Windows-x64.zip` | Portable application; extract the complete archive and keep `sensors` beside the executable. |
| `OrbitWidgets-source.zip` | Packaged project source and accompanying notices. |
| `SHA256SUMS.txt` | SHA-256 checksums for the release downloads. |

Requires Windows 10 version 2004 or newer, or Windows 11, on Intel/AMD x64. Sensor availability depends on hardware, drivers and access rights; unavailable readings appear as **—**. The application binaries are currently unsigned; the official bundled PawnIO installer is signed by its publisher. There is no automatic updater.

Existing settings are preserved. Close Orbit Widgets before updating. Portable users with startup enabled should keep the same application path or disable startup in the old location and enable it again from the new one.

## Русский

Orbit Widgets — прозрачные круглые виджеты производительности для Windows x64 с русским интерфейсом.

**Исправления трея и мониторинга ГП в v1.1.4:** значок трея восстанавливается после перезапуска Проводника, в том числе при повышенном автозапуске, а окна виджетов остаются вне панели задач. Нагрузка ГП предпочтительно берётся из счётчиков Windows WDDM того же однозначно определённого адаптера NVIDIA: используется самый занятый движок, как в диспетчере задач. NVIDIA остаётся источником температуры и явно обозначенным резервным источником нагрузки. Источник нагрузки указан в подсказке.

- Нагрузка и температура ЦП/ГП, занятая оперативная память, занятое место на дисках по буквам, приём и отдача сети в Мбит/с или МБ/с.
- Оформление «Двойной контур» объединяет нагрузку и температуру. Можно выбирать показатели, вертикальное или горизонтальное размещение, размер, расстояние, непрозрачность, одну из четырёх тем, монитор и привязку к экрану.
- Новый профиль размещается вертикально справа по центру дополнительного монитора, если он подключён. Закрепление пропускает клики сквозь виджеты. `Ctrl+Alt+O` открывает настройки, `Ctrl+Alt+L` переключает закрепление.
- Автозапуск при входе в Windows, в том числе с правами администратора через Планировщик заданий.
- Показания NVIDIA поступают через NVML, при необходимости используется модуль LibreHardwareMonitor с доступными ему датчиками. Температура ЦП также может поступать из свежей общей памяти уже запущенного MSI Afterburner. Установщик предлагает официальный подписанный драйвер PawnIO 2.2.0 для прямого доступа к датчикам ЦП.
- Исправлено обрезание подсказок при вертикальном расположении у правого края. Подробности открываются отдельным видимым окном с переносом текста в границах рабочей области монитора, в том числе на экранах с отрицательными координатами.

**Какой файл выбрать:**

| Файл | Назначение |
| --- | --- |
| `OrbitWidgets-Setup-x64.exe` | Установщик Windows x64 с возможностью установки драйвера датчиков ЦП. |
| `OrbitWidgets-Windows-x64.zip` | Переносная программа: распакуйте архив целиком, папка `sensors` должна оставаться рядом с исполняемым файлом. |
| `OrbitWidgets-source.zip` | Архив исходников и сопроводительных лицензий. |
| `SHA256SUMS.txt` | Контрольные суммы SHA-256 для файлов релиза. |

Требуется Windows 10 версии 2004 или новее либо Windows 11 на Intel/AMD x64. Доступность датчиков зависит от оборудования, драйверов и прав; недоступные показания отображаются как **—**. Файлы приложения пока не подписаны; официальный установщик PawnIO подписан его издателем. Автоматического обновления нет.

Существующие настройки сохраняются. Перед обновлением закройте Orbit Widgets. Если у переносной копии включён автозапуск, сохраните путь к программе либо выключите автозапуск в старой папке и включите его из новой.
