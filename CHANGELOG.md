# Changelog / Изменения

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
