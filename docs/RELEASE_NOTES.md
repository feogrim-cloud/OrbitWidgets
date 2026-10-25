# Orbit Widgets v1.1.1

## English

The first public release of Orbit Widgets brings transparent circular performance widgets to Windows x64. The application interface is in Russian.

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

Первый публичный выпуск Orbit Widgets: прозрачные круглые виджеты производительности для Windows x64 с русским интерфейсом.

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
